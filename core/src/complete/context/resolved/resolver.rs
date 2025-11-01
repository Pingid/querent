use crate::ast::AstNode;
use crate::ast::{self};
use crate::complete::context::resolved::ResolvedScope;
use crate::complete::context::resolved::binding::*;
use crate::complete::context::resolved::identifier::*;
use crate::dialect::DialectSpec;
use crate::schema;
use crate::span::Loc;

pub struct ScopeResolver<'a, 'ast> {
    ast: &'ast Loc<ast::Query>,
    text: &'a str,
    schema: &'a schema::Cache,
    spec: &'a DialectSpec,
    resolved: ResolvedScope<'a>,
}

impl<'a, 'ast> ScopeResolver<'a, 'ast> {
    pub fn new(
        text: &'a str, schema: &'a schema::Cache, spec: &'a DialectSpec, ast: &'ast Loc<ast::Query>,
    ) -> Self {
        Self {
            ast,
            text,
            schema,
            spec,
            resolved: ResolvedScope::default(),
        }
    }

    pub fn resolve(mut self) -> ResolvedScope<'a> {
        self.resolve_cte();
        self.resolve_from();
        self.resolve_projected();
        self.resolved
    }

    fn resolve_cte(&mut self) {
        for cte in ast::Cte::find_all_same_query(self.node()) {
            let name = cte.item.name.as_str(self.text);
            let scope = ResolvedScope::build(self.text, self.schema, self.spec, &cte.item.query);
            self.resolved.bind(
                Some(name),
                BindingKind::Cte {
                    name,
                    scope: Box::new(scope),
                },
            );
        }
    }

    fn resolve_from(&mut self) {
        for factor in ast::TableFactor::find_all_same_query(self.node()) {
            match &factor.item {
                ast::TableFactor::Named(named) => {
                    let alias = self.extract_alias(named.alias);
                    let name = self.table_name(&named.name);
                    let table = self.lookup_table(&name);
                    let origin = Some(self.resolved.next_id());
                    let columns: Vec<_> = self
                        .lookup_table_columns(&name)
                        .map(|x| ColumnBinding {
                            dt: Some(x.data_type),
                            col: Some(x),
                            name: ColumnName::from(x),
                            alias,
                            origin,
                        })
                        .collect();
                    self.resolved.bind(
                        alias,
                        BindingKind::Base {
                            name,
                            table,
                            columns,
                        },
                    );
                }
                ast::TableFactor::Function(func) => {
                    let name = self.function_name(&func.name);
                    let alias = self.extract_alias(func.alias);
                    let next_id = self.resolved.next_id();
                    let resolved_function = self.lookup_function(&name);
                    let columns = match resolved_function {
                        Some(def) => match def.return_type() {
                            schema::FunctionReturnType::Table(columns) => columns
                                .iter()
                                .map(|c| ColumnBinding {
                                    dt: Some(c.data_type),
                                    col: None,
                                    name: ColumnName::from(c.column_name.as_str()),
                                    alias,
                                    origin: Some(next_id),
                                })
                                .collect::<Vec<ColumnBinding<'a>>>(),
                            _ => Vec::new(),
                        },
                        None => Vec::new(),
                    };
                    self.resolved.bind(
                        alias,
                        BindingKind::Func {
                            name: name.function_name.unwrap_or(""),
                            definition: resolved_function,
                            columns,
                        },
                    );
                }
                ast::TableFactor::Subquery(subquery) => {
                    let scope =
                        ResolvedScope::build(self.text, self.schema, self.spec, &subquery.query);
                    let alias = self.extract_alias(subquery.alias);
                    self.resolved.bind(
                        alias,
                        BindingKind::Sub {
                            scope: Box::new(scope),
                        },
                    );
                }
                ast::TableFactor::Parenthesized(_parenthesized) => {
                    // println!("parenthesized: {:?}", parenthesized.span.as_str(self.text));
                }
            }
        }
    }

    fn resolve_projected(&mut self) {
        for item in ast::ProjectionItem::find_all_same_query(self.node()) {
            let alias = self.extract_alias(item.alias);
            match &item.expr.item {
                ast::Expr::Name(name) => {
                    let name = self.column_name(name);
                    let mut columns = self.find_columns(&name);

                    if columns.is_empty() && !name.is_star() {
                        if let Some(column) = self.synthetic_projection_column(alias, name) {
                            columns.push(column);
                        }
                    } else {
                        self.apply_single_column_alias(&mut columns, alias, &name);
                    }

                    self.resolved.projected.extend(columns);
                }
                ast::Expr::Literal(item) => {
                    let col = ColumnBinding {
                        dt: self.infer_literal_type(item),
                        col: None,
                        name: ColumnName {
                            column_name: Some(item.span.as_str(self.text)),
                            ..Default::default()
                        },
                        alias,
                        origin: None,
                    };
                    self.resolved.projected.push(col);
                }
                ast::Expr::FunctionCall(func) => {
                    let name = self.function_name(&func.name);
                    let label = alias.unwrap_or(item.span.as_str(self.text));
                    let func = self.lookup_function(&name);

                    let col = ColumnBinding {
                        dt: func.and_then(|f| f.return_type().data_type()),
                        col: None,
                        name: ColumnName::from(label),
                        alias,
                        origin: None,
                    };
                    self.resolved.projected.push(col);
                }
                ast::Expr::Subquery(subquery) => {
                    let scope = ResolvedScope::build(self.text, self.schema, self.spec, &subquery);
                    self.resolved.projected.extend(scope.projected_columns());
                    self.resolved.bind(
                        alias,
                        BindingKind::Sub {
                            scope: Box::new(scope),
                        },
                    );
                }
                ast::Expr::Binary(_)
                | ast::Expr::Unary(_)
                | ast::Expr::Paren(_)
                | ast::Expr::IsNull(_)
                | ast::Expr::Between(_)
                | ast::Expr::Like(_)
                | ast::Expr::ILike(_)
                | ast::Expr::Similar(_)
                | ast::Expr::Array(_)
                | ast::Expr::Quantified(_)
                | ast::Expr::Case(_)
                | ast::Expr::In(_)
                | ast::Expr::Over(_)
                | ast::Expr::Exists(_)
                | ast::Expr::Empty => {
                    let label = alias.unwrap_or(item.span.as_str(self.text));
                    let col = self.expression_column(label, alias);
                    self.resolved.projected.push(col);
                }
            }
        }
    }

    // ---------------- Binding resolution helpers ----------------
    fn match_columns(
        columns: impl Iterator<Item = ColumnBinding<'a>>, column_name: &ColumnName<'a>,
    ) -> impl Iterator<Item = ColumnBinding<'a>> {
        columns
            .filter(|c| column_name.matches_column(&c.name))
            .map(move |c| ColumnBinding {
                dt: c.dt,
                col: c.col,
                name: match column_name.is_star() {
                    true => c.name,
                    false => *column_name,
                },
                alias: c.alias,
                origin: c.origin,
            })
    }

    fn match_scope_columns(
        scope: &ResolvedScope<'a>, column_name: &ColumnName<'a>,
    ) -> Vec<ColumnBinding<'a>> {
        Self::match_columns(scope.projected_columns().iter().copied(), column_name).collect()
    }

    fn find_columns(&self, column_name: &ColumnName<'a>) -> Vec<ColumnBinding<'a>> {
        self.resolved
            .bindings
            .values()
            .flat_map(|binding| self.find_columns_in_binding(column_name, binding))
            .collect()
    }

    fn find_columns_in_binding(
        &self, column_name: &ColumnName<'a>, binding: &Binding<'a>,
    ) -> Vec<ColumnBinding<'a>> {
        match &binding.kind {
            BindingKind::Base { name, columns, .. } => {
                if column_name.matches_table_or_alias(*name, binding.alias) {
                    Self::match_columns(columns.iter().copied(), column_name).collect()
                } else {
                    vec![]
                }
            }
            BindingKind::Cte { scope, name } => {
                let alias = binding.alias.or(Some(*name));
                if column_name.matches_alias(alias) {
                    Self::match_scope_columns(scope, column_name)
                } else {
                    vec![]
                }
            }
            BindingKind::Sub { scope } => {
                if column_name.matches_alias(binding.alias) {
                    Self::match_scope_columns(scope, column_name)
                } else {
                    vec![]
                }
            }
            BindingKind::Func { columns, .. } => {
                if column_name.matches_alias(binding.alias) {
                    Self::match_columns(columns.iter().copied(), column_name).collect()
                } else {
                    vec![]
                }
            }
        }
    }

    // ---------------- AST resolution helpers ----------------
    fn extract_name_parts(&self, name: &Loc<ast::QualifiedName>) -> impl Iterator<Item = &'a str> {
        name.item
            .parts
            .items
            .iter()
            .map(|part| part.span.as_str(self.text))
    }

    fn collect_name_parts(&self, name: &Loc<ast::QualifiedName>) -> Vec<&'a str> {
        self.extract_name_parts(name).collect()
    }

    fn table_name(&self, name: &Loc<ast::QualifiedName>) -> TableName<'a> {
        self.collect_name_parts(name).into()
    }

    fn column_name(&self, name: &Loc<ast::QualifiedName>) -> ColumnName<'a> {
        self.collect_name_parts(name).into()
    }

    fn function_name(&self, name: &Loc<ast::QualifiedName>) -> FunctionName<'a> {
        self.collect_name_parts(name).into()
    }

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

    fn extract_alias(&self, alias: Option<Loc<ast::Identifier>>) -> Option<&'a str> {
        alias.map(|a| a.span.as_str(self.text))
    }

    fn expression_column(&self, label: &'a str, alias: Option<&'a str>) -> ColumnBinding<'a> {
        ColumnBinding {
            dt: None,
            col: None,
            name: ColumnName::from(label),
            alias,
            origin: None,
        }
    }

    fn synthetic_projection_column(
        &self, alias: Option<&'a str>, name: ColumnName<'a>,
    ) -> Option<ColumnBinding<'a>> {
        alias.or(name.column_name).map(|label| ColumnBinding {
            dt: None,
            col: None,
            name,
            alias: Some(label),
            origin: None,
        })
    }

    fn apply_single_column_alias(
        &self, columns: &mut [ColumnBinding<'a>], alias: Option<&'a str>, source: &ColumnName<'a>,
    ) {
        if columns.len() == 1 && !source.is_star() {
            if let Some(alias_str) = alias {
                columns[0].alias = Some(alias_str);
            }
        }
    }

    // ---------------- Schema resolution helpers ----------------
    fn lookup_table(&self, name: &TableName<'a>) -> Option<&'a schema::Table> {
        self.schema
            .get_tables()
            .iter()
            .find(|t| name.matches_table(t))
    }

    fn lookup_table_columns(
        &self, name: &TableName<'a>,
    ) -> impl Iterator<Item = &'a schema::Column> {
        self.schema
            .get_columns()
            .iter()
            .filter(move |c| ColumnName::from(*c).is_from_table(*name))
    }

    fn lookup_function(&self, name: &FunctionName<'a>) -> Option<ResolvedFunction<'a>> {
        self.spec
            .functions
            .values()
            .map(|f| ResolvedFunction::Spec(f))
            .find(|f| Some(f.function_name().as_str()) == name.function_name)
            .or_else(|| {
                self.schema
                    .get_functions()
                    .iter()
                    .map(|f| ResolvedFunction::Schema(f))
                    .find(|f| Some(f.function_name().as_str()) == name.function_name)
            })
    }

    // ---------------- Ast Node traversal ----------------
    fn node(&self) -> ast::Node<'ast> {
        ast::Node::Query(self.ast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complete::context::ParsedStatement;
    use crate::test_util::users_posts_schema;

    fn resolve_with_schema(sql: &str, schema: schema::Cache) -> ResolvedScope<'_> {
        let mut stmt = ParsedStatement::new_ansi_static(sql).unwrap();
        stmt.schema = Box::leak(Box::new(schema));
        ResolvedScope::from(&stmt)
    }

    fn resolve(sql: &str) -> ResolvedScope<'_> {
        resolve_with_schema(sql, users_posts_schema())
    }

    fn assert_projected_in_scope(
        scope: &ResolvedScope<'_>, sql: &str, projected: &[&str],
        data_types: &[Option<schema::DataType>],
    ) {
        let actual_columns: Vec<_> = scope
            .projected_columns()
            .iter()
            .map(|c| c.alias.or_else(|| c.name.column_name).unwrap_or("unknown"))
            .collect();

        for (index, col) in scope.projected_columns().iter().enumerate() {
            let label = col.alias.or_else(|| col.name.column_name);
            assert!(
                scope
                    .projected_columns()
                    .iter()
                    .any(|c| c.alias.or_else(|| c.name.column_name) == label),
                "query: {:?} Expected column {:?} to be projected, but got {:?}",
                sql,
                col,
                actual_columns,
            );
            if let Some(dtype) = data_types.get(index) {
                assert_eq!(
                    col.dt, *dtype,
                    "query: {:?} Expected column {:?} to have data type {:?}, but got {:?}",
                    sql, col, *dtype, col.dt
                );
            }
        }
        assert_eq!(
            scope.projected_columns().iter().count(),
            projected.len(),
            "query: {:?} Expected {} columns to be projected, but got {} columns: {:?}",
            sql,
            projected.len(),
            actual_columns.len(),
            actual_columns,
        );
    }

    fn assert_projected(sql: &str, projected: &[&str], data_types: &[Option<schema::DataType>]) {
        let scope = resolve(sql);
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
        // Subquery with alias
        assert_projected!("SELECT * FROM (SELECT name as user_name FROM foo)", labels: ["user_name"]);

        // Missing column from non-existent table
        assert_projected!("SELECT missing, name FROM users", labels: ["missing", "name"], data_types: [None, Some(Text)]);

        // Simple single column
        assert_projected!("SELECT name FROM users", labels: ["name"], data_types: [Some(Text)]);

        // Star expansion from base table
        assert_projected!("SELECT * FROM users", labels: ["id", "name", "email"], data_types: [Some(Integer), Some(Text), Some(Text)]);

        // // CTE with alias
        assert_projected!("WITH cte as (SELECT name as user_name FROM foo) SELECT * FROM cte", labels: ["user_name"], data_types: [None]);

        // // Multiple columns with aliases
        assert_projected!("SELECT id as user_id, name as user_name, email FROM users", labels: ["user_id", "user_name", "email"], data_types: [Some(Integer), Some(Text), Some(Text)]);

        // // Literal values
        assert_projected!("SELECT 1 as one, 'test' as str, true as flag", labels: ["one", "str", "flag"]);

        // // Mix of columns and literals
        assert_projected!("SELECT id, 'constant' as type, name FROM users", labels: ["id", "type", "name"]);

        // // Table-qualified column names
        assert_projected!("SELECT users.id, users.name FROM users", labels: ["id", "name"]);

        // // Subquery without table alias
        assert_projected!("SELECT * FROM (SELECT id, name FROM users)", labels: ["id", "name"]);

        // // Multiple columns from subquery
        assert_projected!("SELECT * FROM (SELECT id as uid, email as mail FROM users) sub", labels: ["uid", "mail"]);

        // // CTE with multiple columns
        assert_projected!("WITH user_info AS (SELECT id, name FROM users) SELECT * FROM user_info", labels: ["id", "name"]);

        // // Nested subqueries
        assert_projected!("SELECT * FROM (SELECT * FROM (SELECT name FROM users))", labels: ["name"]);

        // // Column from non-existent table (synthetic column)
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

        use crate::test_util::SchemaCacheBuilder;
        let schema = SchemaCacheBuilder::new()
            .add_function(
                "public",
                "foo",
                schema::FunctionReturnType::Table(vec![schema::TableColumn {
                    column_name: "name".to_string(),
                    data_type: schema::DataType::Integer,
                }]),
                &[schema::DataType::Text],
            )
            .add_function(
                "public",
                "bar",
                schema::FunctionReturnType::Table(vec![
                    schema::TableColumn {
                        column_name: "baz".to_string(),
                        data_type: schema::DataType::Integer,
                    },
                    schema::TableColumn {
                        column_name: "biz".to_string(),
                        data_type: schema::DataType::Integer,
                    },
                ]),
                &[schema::DataType::Text],
            )
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

    #[test]
    fn test_subquery_resolve() {
        // Test basic subquery resolution
        let sql = "SELECT * FROM (SELECT email as u_email FROM users) u";
        let scope = resolve(sql);
        let columns = scope.projected_columns();
        assert_eq!(columns.len(), 1);
        // Check that we get u_email as the alias
        assert_eq!(
            columns[0].alias.or(columns[0].name.column_name),
            Some("u_email")
        );
    }
}
