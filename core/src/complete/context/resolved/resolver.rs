use crate::ast::AstNode;
use crate::ast::{self};
use crate::complete::context::resolved::ResolvedScope;
use crate::complete::context::resolved::binding::*;
use crate::complete::context::resolved::identifier::*;
// use crate::dialect::DialectSpec;
use crate::schema;
use crate::span::Loc;

pub struct ScopeResolver<'a> {
    ast: &'a Loc<ast::Query>,
    text: &'a str,
    schema: &'a schema::Cache,
    // spec: &'a DialectSpec,
    resolved: ResolvedScope<'a>,
}

impl<'a> ScopeResolver<'a> {
    pub fn new(text: &'a str, schema: &'a schema::Cache, ast: &'a Loc<ast::Query>) -> Self {
        Self {
            ast,
            text,
            schema,
            // spec,
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
            let scope = ResolvedScope::build(self.text, self.schema, &cte.item.query);
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
                    let name = self
                        .extract_name_parts(&named.name)
                        .collect::<Vec<_>>()
                        .into();
                    let table = self.lookup_table(&name);
                    let origin = Some(ColumnOrigin::Binding(self.resolved.next_id()));
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
                    // println!("func: {:?}", func.span.as_str(self.text));
                }
                ast::TableFactor::Subquery(subquery) => {
                    let scope = ResolvedScope::build(self.text, self.schema, &subquery.query);
                    let alias = self.extract_alias(subquery.alias);
                    self.resolved.bind(
                        alias,
                        BindingKind::Sub {
                            scope: Box::new(scope),
                        },
                    );
                }
                ast::TableFactor::Parenthesized(parenthesized) => {
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
                    let name = self.extract_name_parts(name).collect::<Vec<_>>().into();
                    let mut columns = self.find_columns(&name);

                    // If we didn't find any columns and this is not a star, create a synthetic column
                    if columns.is_empty() && !name.is_star() {
                        let col_name = alias.or(name.column_name);
                        // self.resolved.bindings.iter().find_map(|(_, binding)|
                        if let Some(col_name) = col_name {
                            columns.push(ColumnBinding {
                                dt: None,
                                col: None,
                                name,
                                alias: Some(col_name),
                                origin: None,
                            });
                        }
                    } else if let Some(alias_str) = alias {
                        // If we have an alias and this is not a star expansion, apply it
                        if !name.is_star() && columns.len() == 1 {
                            columns[0].alias = Some(alias_str);
                        }
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
                    let name: FunctionName<'a> = self
                        .extract_name_parts(&func.name)
                        .collect::<Vec<_>>()
                        .into();

                    let label = alias.unwrap_or(item.span.as_str(self.text));
                    // let func = self.lookup_function(&name);
                    // let col = ColumnBinding {
                    //     dt: func.and_then(|f| f.return_type().data_type()),
                    //     col: None,
                    //     name: ColumnName::from(label),
                    //     alias,
                    //     origin: func.map(ColumnOrigin::Func),
                    // };
                    // self.resolved.projected.push(col);
                }
                _ => {
                    // Other expressions not yet supported
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
                // CTEs are referenced by their name (which acts as an alias)
                if column_name.is_unqualified() || column_name.table_name == Some(*name) {
                    Self::match_columns(scope.projected_columns().iter().copied(), column_name)
                        .collect()
                } else {
                    vec![]
                }
            }
            BindingKind::Sub { scope } => {
                // Subqueries can only be referenced via their alias
                if column_name.is_unqualified() || column_name.table_name == binding.alias {
                    Self::match_columns(scope.projected_columns().iter().copied(), column_name)
                        .collect()
                } else {
                    vec![]
                }
            }
            BindingKind::Func { name, definition } => {
                let name = FunctionName::from(*name);
                // let func = self.lookup_function(&name);
                println!("func: {:?}", column_name);
                println!("definition: {:?}", definition);
                // println!("definition: {:?}", func);
                // TODO: implement function column resolution
                vec![]
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

    // fn lookup_function(&self, name: &FunctionName<'a>) -> Option<ResolvedFunction<'a>> {
    //     self.spec
    //         .functions
    //         .values()
    //         .map(|f| ResolvedFunction::Spec(f))
    //         .find(|f| Some(f.function_name().as_str()) == name.function_name)
    //         .or_else(|| {
    //             self.schema
    //                 .get_functions()
    //                 .iter()
    //                 .map(|f| ResolvedFunction::Schema(f))
    //                 .find(|f| Some(f.function_name().as_str()) == name.function_name)
    //         })
    // }

    // ---------------- Ast Node traversal ----------------
    fn node(&self) -> ast::Node<'a> {
        ast::Node::Query(self.ast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::ansi;
    use crate::lex::lex;
    use crate::parse::Parser;
    use crate::test_util::users_posts_schema;

    fn parse_query(sql: &str) -> ast::Query {
        let tokens = lex(&ansi::SPEC, sql);
        let mut parser = Parser::new(&tokens);
        let stmt = parser.parse_statement().unwrap().item;
        match stmt {
            ast::Statement::Query(q) => q.item,
            _ => panic!("Expected query statement"),
        }
    }

    fn resolve(sql: &str) -> ResolvedScope<'_> {
        let sql = Box::leak(sql.to_string().into_boxed_str());
        let schema = Box::leak(Box::new(users_posts_schema()));
        let query = parse_query(sql);
        let query_loc = Box::leak(Box::new(Loc {
            span: crate::span::Span::new(0, sql.len()),
            item: query,
        }));
        ResolvedScope::build(sql, schema, query_loc)
    }

    fn assert_projected(sql: &str, projected: &[&str]) {
        let scope = resolve(sql);
        let actual_columns: Vec<_> = scope
            .projected_columns()
            .iter()
            .map(|c| c.alias.or_else(|| c.name.column_name).unwrap_or("unknown"))
            .collect();

        for col in projected {
            assert!(
                scope
                    .projected_columns()
                    .iter()
                    .any(|c| c.alias.or_else(|| c.name.column_name) == Some(col)),
                "query: {:?} Expected column {:?} to be projected, but got {:?}",
                sql,
                col,
                actual_columns,
            );
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

    #[test]
    fn test_projected() {
        // Subquery with alias
        let sql = "SELECT * FROM (SELECT name as user_name FROM foo)";
        assert_projected(&sql, &["user_name"]);

        // Missing column from non-existent table
        let sql = "SELECT missing, name FROM users";
        assert_projected(&sql, &["missing", "name"]);

        // Simple single column
        let sql = "SELECT name FROM users";
        assert_projected(&sql, &["name"]);

        // Star expansion from base table
        let sql = "SELECT * FROM users";
        assert_projected(&sql, &["id", "name", "email"]);

        // CTE with alias
        let sql = "WITH cte as (SELECT name as user_name FROM foo) SELECT * FROM cte";
        assert_projected(&sql, &["user_name"]);

        // Multiple columns with aliases
        let sql = "SELECT id as user_id, name as user_name, email FROM users";
        assert_projected(&sql, &["user_id", "user_name", "email"]);

        // Literal values
        let sql = "SELECT 1 as one, 'test' as str, true as flag";
        assert_projected(&sql, &["one", "str", "flag"]);

        // Mix of columns and literals
        let sql = "SELECT id, 'constant' as type, name FROM users";
        assert_projected(&sql, &["id", "type", "name"]);

        // Table-qualified column names
        let sql = "SELECT users.id, users.name FROM users";
        assert_projected(&sql, &["id", "name"]);

        // Subquery without table alias
        let sql = "SELECT * FROM (SELECT id, name FROM users)";
        assert_projected(&sql, &["id", "name"]);

        // Multiple columns from subquery
        let sql = "SELECT * FROM (SELECT id as uid, email as mail FROM users) sub";
        assert_projected(&sql, &["uid", "mail"]);

        // CTE with multiple columns
        let sql = "WITH user_info AS (SELECT id, name FROM users) SELECT * FROM user_info";
        assert_projected(&sql, &["id", "name"]);

        // Nested subqueries
        let sql = "SELECT * FROM (SELECT * FROM (SELECT name FROM users))";
        assert_projected(&sql, &["name"]);

        // Column from non-existent table (synthetic column)
        let sql = "SELECT fake_col FROM non_existent_table";
        assert_projected(&sql, &["fake_col"]);

        let sql = "SELECT posts.id FROM users, posts";
        assert_projected(&sql, &["id"]);

        let sql = "SELECT UPPER(name) FROM users";
        assert_projected(&sql, &["UPPER(name)"]);

        let sql = "SELECT UPPER(name) as upper_name FROM users";
        assert_projected(&sql, &["upper_name"]);
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
