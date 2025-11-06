use crate::ast::AstNode;
use crate::ast::{self};
use crate::complete::context::scope::binding::*;
use crate::complete::context::scope::identifier::IdentKind;
use crate::complete::context::scope::identifier::QualifiedIdent;
use crate::dialect::DialectSpec;
use crate::schema;
use crate::span::Loc;
use crate::span::Span;

pub struct ScopeGraphBuilder<'a, 'ast> {
    ast: &'ast Loc<ast::Query>,
    text: &'a str,
    schema: &'a schema::Cache,
    spec: &'a DialectSpec,
    graph: Scope<'a>,
}

impl ScopeGraphBuilder<'_, '_> {
    pub fn build_graph<'a, 'ast>(
        text: &'a str, schema: &'a schema::Cache, spec: &'a DialectSpec, ast: &'ast Loc<ast::Query>,
    ) -> Scope<'a> {
        ScopeGraphBuilder::new(text, schema, spec, ast).resolve()
    }
}

impl<'a, 'ast> ScopeGraphBuilder<'a, 'ast> {
    /// Creates a new scope resolver for the given query AST
    pub fn new(
        text: &'a str, schema: &'a schema::Cache, spec: &'a DialectSpec, ast: &'ast Loc<ast::Query>,
    ) -> Self {
        Self {
            ast,
            text,
            schema,
            spec,
            graph: Scope::default(),
        }
    }

    /// Resolves all CTEs, FROM clauses, and projections in the query
    pub fn resolve(mut self) -> Scope<'a> {
        self.resolve_cte();
        self.resolve_from();
        self.resolve_projected();
        self.graph
    }

    /// Resolves Common Table Expressions (CTEs) in the WITH clause
    fn resolve_cte(&mut self) {
        for cte in ast::Cte::find_all_same_query(self.node()) {
            let name = self.extract_text(cte.item.name);
            let scope = Self::build_graph(self.text, self.schema, self.spec, &cte.item.query);
            let available = scope
                .projected()
                .iter()
                .map(|p| p.propagate(Some(name)))
                .collect();
            self.graph.bind(
                Some(name),
                BindKind::Cte {
                    name,
                    scope: Box::new(scope),
                },
                available,
            );
        }
    }

    /// Resolves all table references in the FROM clause
    fn resolve_from(&mut self) {
        for factor in ast::TableFactor::find_all_same_query(self.node()) {
            self.resolve_table_factor(&factor);
        }
    }

    /// Resolves all projected columns in the SELECT clause
    fn resolve_projected(&mut self) {
        for item in ast::ProjectionItem::find_all_same_query(self.node()) {
            let alias = self.extract_alias(item.alias);
            let projections = self.resolve_projection_item(&item, alias);
            self.graph.projected.extend(projections);
        }
    }

    /// Dispatches table factor resolution based on its type
    fn resolve_table_factor(&mut self, factor: &Loc<ast::TableFactor>) {
        match &factor.item {
            ast::TableFactor::Named(named) => self.resolve_table_named(named),
            ast::TableFactor::Function(func) => self.resolve_table_function(func),
            ast::TableFactor::Subquery(subquery) => self.resolve_table_subquery(subquery),
            ast::TableFactor::Parenthesized(_parenthesized) => {
                // TODO: Handle parenthesized table factors if needed
            }
        }
    }

    /// Resolves a named table reference (e.g., users, schema.table)
    fn resolve_table_named(&mut self, named: &ast::NamedTableFactor) {
        let alias = self.extract_alias(named.alias);
        let name = self.table_name(&named.name);
        let table = self.lookup_table(&name);
        let id = self.graph.new_bind_id();

        let label = |c: &'a schema::Column| match alias {
            Some(alias) => {
                QualifiedIdent::from_slice(IdentKind::Column, &[alias, c.column_name.as_str()])
                    .unwrap()
            }
            None => QualifiedIdent::from(c),
        };

        let available = self
            .lookup_schema_columns(&name)
            .map(|c| Projection {
                label: label(c),
                kind: ProjectionKind::Column {
                    source: Some(id),
                    schema_column: Some(c),
                },
            })
            .collect();

        self.graph.bind(
            alias.or(Some(name.name())),
            BindKind::Base { name, table },
            available,
        );
    }

    /// Resolves a table-valued function (e.g., generate_series(), json_table())
    fn resolve_table_function(&mut self, func: &ast::FunctionTableFactor) {
        let name = self.function_name(&func.name);
        let alias = self.extract_alias(func.alias);
        let resolved_function = self.lookup_function(&name);
        let columns = self.extract_function_columns(&resolved_function);
        let id = self.graph.new_bind_id();

        let label = |c: &'a schema::TableColumn| match alias {
            Some(alias) => {
                QualifiedIdent::from_slice(IdentKind::Column, &[alias, c.column_name.as_str()])
                    .unwrap()
            }
            None => {
                QualifiedIdent::from_slice(IdentKind::Column, &[c.column_name.as_str()]).unwrap()
            }
        };

        let available = columns
            .iter()
            .map(|c| Projection {
                label: label(c),
                kind: ProjectionKind::TableFunction {
                    source: id,
                    column: c,
                    definition: resolved_function,
                },
            })
            .collect();

        self.graph.bind(
            alias,
            BindKind::Func {
                name: name.name(),
                definition: resolved_function,
                columns,
            },
            available,
        );
    }

    /// Resolves a subquery used as a table source
    fn resolve_table_subquery(&mut self, subquery: &ast::SubqueryTableFactor) {
        let scope = Self::build_graph(self.text, self.schema, self.spec, &subquery.query);
        let alias = self.extract_alias(subquery.alias);
        let available = scope
            .projected()
            .iter()
            .map(|p| p.propagate(alias))
            .collect();
        self.graph.bind(
            alias,
            BindKind::Sub {
                scope: Box::new(scope),
            },
            available,
        );
    }

    /// Extracts column definitions from a table-valued function
    fn extract_function_columns(
        &self, func_ref: &Option<FunctionRef<'a>>,
    ) -> Vec<&'a schema::TableColumn> {
        match func_ref {
            Some(def) => match def.return_type() {
                schema::FuncReturnType::Table(columns) => columns.iter().collect(),
                _ => Vec::new(),
            },
            None => Vec::new(),
        }
    }

    /// Resolves a single projection item based on its expression type
    fn resolve_projection_item(
        &mut self, item: &Loc<ast::ProjectionItem>, alias: Option<&'a str>,
    ) -> Vec<Projection<'a>> {
        match &item.expr.item {
            ast::Expr::Name(name) => self.resolve_name_projection(name, alias),
            ast::Expr::Literal(lit) => vec![self.resolve_literal_projection(lit, alias)],
            ast::Expr::FunctionCall(func) => {
                vec![self.resolve_function_projection(func, alias, self.extract_text(item))]
            }
            ast::Expr::Subquery(subquery) => self.resolve_subquery_projection(subquery, alias),
            _ => vec![self.resolve_expression_projection(self.extract_text(item), alias)],
        }
    }

    /// Resolves column name projections, including wildcards and qualified names
    fn resolve_name_projection(
        &self, name: &Loc<ast::QualifiedName>, alias: Option<&'a str>,
    ) -> Vec<Projection<'a>> {
        let column_name = self.column_name(name);

        // Handle wildcard expansion
        if column_name.is_wildcard() {
            return self.expand_wildcard(&column_name);
        }

        // Try to resolve from existing bindings, schema, or create synthetic
        self.resolve_single_column(&column_name, alias)
            .map(|p| vec![p])
            .unwrap_or_default()
    }

    /// Expands wildcard projections (e.g., *, table.*)
    fn expand_wildcard(&self, column_name: &QualifiedIdent<'a>) -> Vec<Projection<'a>> {
        self.graph
            .available()
            .filter(|p| match (column_name.table(), p.label.table()) {
                (Some(table), Some(label_table)) => table == label_table,
                _ => true,
            })
            .map(|p| *p)
            .collect()
    }

    /// Resolves a single column from bindings, schema, or creates a synthetic projection
    fn resolve_single_column(
        &self, column_name: &QualifiedIdent<'a>, alias: Option<&'a str>,
    ) -> Option<Projection<'a>> {
        // First, try to resolve from existing bindings
        self.resolve_from_bindings(column_name, alias)
            // If no bindings, try schema lookup (only when no bindings exist)
            .or_else(|| self.resolve_from_schema_if_no_bindings(column_name))
            // Finally, create a synthetic projection
            .or_else(|| self.synthetic_projection_column(alias, *column_name))
    }

    /// Attempts to resolve a column from existing table bindings
    fn resolve_from_bindings(
        &self, column_name: &QualifiedIdent<'a>, alias: Option<&'a str>,
    ) -> Option<Projection<'a>> {
        // If column has a table qualifier, look for that specific binding
        let binding = if let Some(table) = column_name.table() {
            self.graph.get_bind_by_alias(table)
        } else {
            // Otherwise, find any binding that has this column
            self.find_binding_by_column_name(column_name.name())
        };

        // Find the matching column in the binding's available projections
        binding.and_then(|(_, bind)| {
            bind.available
                .iter()
                .find(|p| p.label.name() == column_name.name())
                .and_then(|p| p.project(*column_name, alias))
        })
    }

    /// Resolves from schema when no table bindings exist (for simple queries without FROM)
    fn resolve_from_schema_if_no_bindings(
        &self, column_name: &QualifiedIdent<'a>,
    ) -> Option<Projection<'a>> {
        // Only attempt schema lookup if there are no bindings
        // (e.g., "SELECT title" without FROM clause)
        if self.graph.bindings.is_empty() {
            self.lookup_schema_columns(column_name)
                .next()
                .map(|c| Projection {
                    label: QualifiedIdent::from(c),
                    kind: ProjectionKind::Column {
                        source: None,
                        schema_column: Some(c),
                    },
                })
        } else {
            None
        }
    }

    /// Resolves literal value projections (e.g., 1, 'text', true)
    fn resolve_literal_projection(
        &self, literal: &Loc<ast::Literal>, alias: Option<&'a str>,
    ) -> Projection<'a> {
        let label = alias.unwrap_or(self.extract_text(literal));
        Projection {
            label: QualifiedIdent::from_str(IdentKind::Column, label),
            kind: ProjectionKind::Literal {
                alias,
                text: self.extract_text(literal),
                data_type: self.infer_literal_type(literal),
            },
        }
    }

    /// Resolves scalar function call projections (e.g., UPPER(name), COUNT(*))
    fn resolve_function_projection(
        &self, func: &ast::FunctionCall, alias: Option<&'a str>, full_expr: &'a str,
    ) -> Projection<'a> {
        let name = self.function_name(&func.name);
        let label = alias.unwrap_or(full_expr);
        let func_def = self.lookup_function(&name);

        Projection {
            label: QualifiedIdent::from_str(IdentKind::Column, label),
            kind: ProjectionKind::ScalarFunction {
                name: name.name(),
                alias,
                return_type: func_def.and_then(|f| f.return_type().data_type()),
            },
        }
    }

    /// Resolves subquery projections in SELECT clause
    fn resolve_subquery_projection(
        &mut self, subquery: &Loc<ast::Query>, alias: Option<&'a str>,
    ) -> Vec<Projection<'a>> {
        let scope = Self::build_graph(self.text, self.schema, self.spec, subquery);
        scope
            .projected()
            .first()
            .and_then(|p| p.project(QualifiedIdent::from_str(IdentKind::Column, "*"), alias))
            .map(|p| vec![p])
            .unwrap_or_default()
    }

    /// Resolves complex expression projections (e.g., CASE, operators)
    fn resolve_expression_projection(
        &self, expr_text: &'a str, alias: Option<&'a str>,
    ) -> Projection<'a> {
        let label = alias.unwrap_or(expr_text);
        self.expression_column(label, alias)
    }

    // ================== COLUMN FINDING AND BINDING ==================
    fn find_binding_by_column_name(&self, name: &str) -> Option<&(BindId, Bind<'a>)> {
        self.graph
            .bindings
            .iter()
            .find(|(_, bind)| bind.available.iter().any(|p| p.label.name() == name))
    }

    /// Creates a projection for a generic expression
    fn expression_column(&self, label: &'a str, alias: Option<&'a str>) -> Projection<'a> {
        Projection {
            label: QualifiedIdent::from_str(IdentKind::Column, alias.unwrap_or(label)),
            kind: ProjectionKind::Expression {
                data_type: None,
                alias,
            },
        }
    }

    /// Creates a synthetic projection for unresolved columns
    fn synthetic_projection_column(
        &self, alias: Option<&'a str>, name: QualifiedIdent<'a>,
    ) -> Option<Projection<'a>> {
        alias.or_else(|| Some(name.name())).map(|label| Projection {
            label: QualifiedIdent::from_str(IdentKind::Column, label),
            kind: ProjectionKind::Unknown,
        })
    }

    /// Converts a qualified name AST node to a table identifier
    fn table_name(&self, name: &Loc<ast::QualifiedName>) -> QualifiedIdent<'a> {
        let parts = self.collect_name_parts(name);
        QualifiedIdent::from_slice(IdentKind::Table, &parts).unwrap_or(QualifiedIdent {
            kind: IdentKind::Table,
            name: "",
            database: None,
            schema: None,
            parent: None,
        })
    }

    /// Converts a qualified name AST node to a column identifier
    fn column_name(&self, name: &Loc<ast::QualifiedName>) -> QualifiedIdent<'a> {
        let parts = self.collect_name_parts(name);
        QualifiedIdent::from_slice(IdentKind::Column, &parts).unwrap_or(QualifiedIdent {
            kind: IdentKind::Column,
            name: "",
            database: None,
            schema: None,
            parent: None,
        })
    }

    /// Converts a qualified name AST node to a function identifier
    fn function_name(&self, name: &Loc<ast::QualifiedName>) -> QualifiedIdent<'a> {
        let parts = self.collect_name_parts(name);
        QualifiedIdent::from_slice(IdentKind::Function, &parts).unwrap_or(QualifiedIdent {
            kind: IdentKind::Function,
            name: "",
            database: None,
            schema: None,
            parent: None,
        })
    }

    /// Infers the data type of a literal value
    fn infer_literal_type(&self, literal: &Loc<ast::Literal>) -> Option<schema::DataType> {
        match &literal.item {
            ast::Literal::Number(_) => Some(schema::DataType::Integer),
            ast::Literal::Boolean(_) => Some(schema::DataType::Boolean),
            ast::Literal::String(_) => Some(schema::DataType::Text),
            ast::Literal::Null => Some(schema::DataType::Null),
            ast::Literal::Float(_) => Some(schema::DataType::Float),
            ast::Literal::TypedString {
                data_type: _,
                value: _,
            } => None,
        }
    }

    /// Looks up a table in the schema cache
    fn lookup_table(&self, name: &QualifiedIdent<'a>) -> Option<&'a schema::Table> {
        self.schema
            .get_tables()
            .iter()
            .find(|t| name.matches(&QualifiedIdent::from(*t)))
    }

    /// Looks up columns for a table in the schema cache
    fn lookup_schema_columns(
        &self, name: &QualifiedIdent<'a>,
    ) -> impl Iterator<Item = &'a schema::Column> {
        self.schema
            .get_columns()
            .iter()
            .filter(move |c| QualifiedIdent::from(*c).matches(name))
    }

    /// Looks up a function in the dialect spec or schema cache
    fn lookup_function(&self, name: &QualifiedIdent<'a>) -> Option<FunctionRef<'a>> {
        self.spec
            .functions
            .values()
            .map(|f| FunctionRef::Spec(f))
            .find(|f| f.function_name() == name.name())
            .or_else(|| {
                self.schema
                    .get_functions()
                    .iter()
                    .map(|f| FunctionRef::Schema(f))
                    .find(|f| f.function_name() == name.name())
            })
    }

    /// Extracts an optional alias from an identifier
    fn extract_alias(&self, alias: Option<Loc<ast::Identifier>>) -> Option<&'a str> {
        alias.map(|a| self.extract_text(&a))
    }

    /// Collects name parts into a vector
    fn collect_name_parts(&self, name: &Loc<ast::QualifiedName>) -> Vec<&'a str> {
        self.extract_name_parts(name).collect()
    }

    /// Extracts name parts from a qualified name AST node
    fn extract_name_parts(&self, name: &Loc<ast::QualifiedName>) -> impl Iterator<Item = &'a str> {
        name.item
            .parts
            .items
            .iter()
            .map(|part| self.extract_text(part))
    }

    /// Extracts an optional alias from an identifier
    fn extract_text(&self, name: impl Into<Span>) -> &'a str {
        name.into().as_str(self.text)
    }

    /// Returns the root AST node for traversal
    fn node(&self) -> ast::Node<'ast> {
        ast::Node::Query(self.ast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complete::context::ParsedStatement;
    use crate::schema::CacheBuilder;
    use crate::test_utils::posts_schema;
    use crate::test_utils::users_schema;

    fn resolve_with_schema(sql: &str, schema: schema::Cache) -> Scope<'_> {
        let mut stmt = ParsedStatement::new_ansi_static(sql).unwrap();
        stmt.schema = Box::leak(Box::new(schema));
        Scope::from(&stmt)
    }

    fn assert_projected_in_scope(
        scope: &Scope<'_>, sql: &str, projected: &[&str], data_types: &[Option<schema::DataType>],
    ) {
        let actual_columns: Vec<_> = scope.projected.iter().map(|c| c.label.name()).collect();
        // println!("actual_columns: {:#?}", scope.projected);
        assert_eq!(
            actual_columns, projected,
            "query: {:?}\nExpected columns to be projected: {:?}\nBut got: {:?}",
            sql, projected, actual_columns
        );

        if data_types.len() > 0 {
            let actual_data_types: Vec<_> = scope.projected.iter().map(|c| c.data_type()).collect();
            assert_eq!(
                actual_data_types, data_types,
                "query: {:?}\nExpected data types to be projected: {:?}\nBut got: {:?}",
                sql, data_types, actual_data_types
            );
        }
    }

    fn assert_projected(sql: &str, projected: &[&str], data_types: &[Option<schema::DataType>]) {
        let scope = resolve_with_schema(sql, users_schema().combine(posts_schema()));
        assert_projected_in_scope(&scope, sql, projected, data_types);
    }

    macro_rules! assert_projected {
        ($text:expr, labels: [$($label:expr),* $(,)?], data_types: [$($dtype:expr),* $(,)?]) => {{
            assert_projected($text, &[$($label),*], &[$($dtype),*]);
        }};
        ($text:expr, labels: [$($label:expr),* $(,)?]) => {{
            assert_projected($text, &[$($label),*], &[]);
        }};
    }

    #[test]
    fn test_projected() {
        use schema::DataType::*;
        // Infer from schema when no binding is found
        assert_projected!("SELECT title, ", labels: ["title"], data_types: [Some(Text)]);

        // Wildcard expansion from base table
        assert_projected!("SELECT users.* FROM users, posts", labels: ["id", "name", "email"]);

        // Subquery with alias
        assert_projected!("SELECT * FROM (SELECT name as user_name FROM foo)", labels: ["user_name"]);

        // Missing column from non-existent table
        assert_projected!("SELECT missing, name FROM users", labels: ["missing", "name"], data_types: [None, Some(Text)]);

        // Simple single column
        assert_projected!("SELECT name FROM users", labels: ["name"], data_types: [Some(Text)]);

        // Star expansion from base table
        assert_projected!("SELECT * FROM users", labels: ["id", "name", "email"], data_types: [Some(Integer), Some(Text), Some(Text)]);

        // CTE with alias
        assert_projected!("WITH cte as (SELECT name as user_name FROM foo) SELECT * FROM cte", labels: ["user_name"], data_types: [None]);

        // Multiple columns with aliases
        assert_projected!("SELECT id as user_id, name as user_name, email FROM users", labels: ["user_id", "user_name", "email"], data_types: [Some(Integer), Some(Text), Some(Text)]);

        // Literal values
        assert_projected!("SELECT 1 as one, 'test' as str, true as flag", labels: ["one", "str", "flag"]);

        // Mix of columns and literals
        assert_projected!("SELECT id, 'constant' as type, name FROM users", labels: ["id", "type", "name"]);

        // Table-qualified column names
        assert_projected!("SELECT users.id, users.name FROM users", labels: ["id", "name"]);

        // Subquery without table alias
        assert_projected!("SELECT * FROM (SELECT id, name FROM users)", labels: ["id", "name"]);

        // Multiple columns from subquery
        assert_projected!("SELECT * FROM (SELECT id as uid, email as mail FROM users) sub", labels: ["uid", "mail"]);

        // CTE with multiple columns
        assert_projected!("WITH user_info AS (SELECT id, name FROM users) SELECT * FROM user_info", labels: ["id", "name"]);

        // Nested subqueries
        assert_projected!("SELECT * FROM (SELECT * FROM (SELECT name FROM users))", labels: ["name"]);

        // Column from non-existent table (synthetic column)
        assert_projected!("SELECT fake_col FROM non_existent_table", labels: ["fake_col"]);

        assert_projected!("SELECT posts.id FROM users, posts", labels: ["id"], data_types: [Some(Integer)]);
        assert_projected!("SELECT UPPER(name) FROM users", labels: ["UPPER(name)"], data_types: [Some(Text)]);
        assert_projected!("SELECT UPPER(name) as upper_name FROM users", labels: ["upper_name"], data_types: [Some(Text)]);
        assert_projected!("SELECT id + 1 FROM users", labels: ["id + 1"], data_types: [None]);
        assert_projected!("SELECT id + 1 AS plus_one FROM users", labels: ["plus_one"], data_types: [None]);

        assert_projected!("SELECT CASE WHEN id > 0 THEN 'pos' ELSE 'neg' END AS status FROM users", labels: ["status"], data_types: [None]);

        assert_projected!("SELECT NOT (id > 0) FROM users", labels: ["NOT (id > 0)"]);

        assert_projected!("SELECT (SELECT COUNT(*) FROM posts) AS post_count FROM users", labels: ["post_count"], data_types: [Some(Integer)]);
    }

    #[test]
    fn test_function_projection() {
        use schema::DataType::*;
        let schema = CacheBuilder::new()
            .table_function("foo", &[Text], vec![("name", Integer)])
            .table_function("bar", &[Text], vec![("baz", Integer), ("biz", Integer)])
            .build();
        let sql = "SELECT * FROM foo()";
        assert_projected_in_scope(
            &resolve_with_schema(sql, schema.clone()),
            sql,
            &["name"],
            &[Some(Integer)],
        );

        let sql = "SELECT * FROM (SELECT baz FROM bar()) u";
        assert_projected_in_scope(
            &resolve_with_schema(sql, schema.clone()),
            sql,
            &["baz"],
            &[Some(Integer)],
        );
    }
}
