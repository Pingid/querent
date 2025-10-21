use crate::ast::{self, QualifiedName};
use crate::span::Loc;

use super::*;

pub struct ScopeBuilder<'txt> {
    pub text: &'txt str,
    pub position: usize,
    pub ast: ast::Node<'txt>,
}

impl<'txt> ScopeBuilder<'txt> {
    pub fn new(text: &'txt str, position: usize, ast: ast::Node<'txt>) -> Self {
        Self {
            text,
            position,
            ast,
        }
    }

    pub fn build(&self) -> Scope {
        let query = self.ast.find_rev(
            |node| matches!(node, ast::Node::Query(q) if q.span.contains_inclusive(self.position) || (q.span.end <= self.position && self.text[q.span.end..self.position].chars().all(|c| c.is_whitespace()))),
        );

        let Some(query_node) = query else {
            return Scope::default();
        };
        self.build_scope(query_node)
    }

    fn build_scope(&self, query_node: impl Into<ast::Node<'txt>>) -> Scope {
        let mut scope = Scope::default();
        let query_node = query_node.into();
        self.gather_ctes(&mut scope, query_node);
        self.gather_relations(&mut scope, query_node);
        self.gather_projections(&mut scope, query_node);
        self.gather_group_by(&mut scope, query_node);
        self.gather_order_by(&mut scope, query_node);
        scope
    }

    fn gather_ctes(&self, scope: &mut Scope, query_node: ast::Node<'_>) {
        let ast::Node::Query(query) = query_node else {
            return;
        };

        let Some(with) = &query.item.with else {
            return;
        };

        // Process each CTE
        for cte in &with.item.ctes {
            let name = self.span_string(&cte.item.name);
            let cte_scope = self.build_scope(&*cte.item.query);
            scope.insert_relation(RelationKind::Cte(Box::new(cte_scope)), Some(name));
        }
    }

    fn gather_relations(&self, scope: &mut Scope, query_node: ast::Node<'_>) {
        let Some(select) = query_node.as_select() else {
            return;
        };

        let Some(from) = &select.from else {
            return;
        };

        // Collect all table factors from the FROM clause (including tables in JOINs)
        // Process each table reference (which may contain JOINs)
        for table_ref in &from.item.sources.items {
            self.gather_table_ref(scope, table_ref);
        }
    }

    fn gather_table_ref(&self, scope: &mut Scope, table_ref: &Loc<ast::TableRef>) {
        match &table_ref.item {
            ast::TableRef::Factor(factor) => {
                self.gather_table_factor(scope, factor);
            }
            ast::TableRef::Join(join) => {
                // Process left side of join
                self.gather_table_ref(scope, &join.item.left);
                // Process right side of join
                self.gather_table_ref(scope, &join.item.right);
            }
        }
    }

    fn gather_table_factor(&self, scope: &mut Scope, factor: &Loc<ast::TableFactor>) {
        match &factor.item {
            ast::TableFactor::Named(n) => {
                let path = self.name_path(&n.item.name);
                let alias = n.item.alias.as_ref().map(|a| self.span_string(&a.span));

                // Check if this references a CTE
                if path.0.len() == 1 && scope.by_name.contains_key(&path.0[0]) {
                    // This is a reference to an existing CTE
                    if let Some(a) = alias
                        && let Some(&cte_id) = scope.by_name.get(&path.0[0])
                    {
                        scope.by_name.insert(a, cte_id);
                    }
                } else {
                    // Regular table
                    scope.insert_relation(RelationKind::Base(path), alias);
                }
            }
            ast::TableFactor::Subquery(n) => {
                let alias = n.item.alias.as_ref().map(|a| self.span_string(&a.span));
                let kind = RelationKind::Subquery(Box::new(self.build_scope(&n.item.query)));
                scope.insert_relation(kind, alias);
            }
            ast::TableFactor::Parenthesized(table_ref) => {
                self.gather_table_ref(scope, table_ref);
            }
            ast::TableFactor::Function(_) => {
                // TODO: Handle table functions
            }
        }
    }

    fn gather_projections(&self, scope: &mut Scope, query_node: ast::Node<'_>) {
        let Some(select) = query_node.as_select() else {
            return;
        };

        for item in &select.projection.items {
            let name = item
                .alias
                .as_ref()
                .map(|a| self.span_string(&a.span))
                .unwrap_or_else(|| self.projection_name(&item.expr));

            let qualifier = self.get_qualifier_from_expr(&item.expr);
            scope.insert_column(name, self.column_origin(scope, &item.expr), qualifier);
        }
    }

    fn gather_group_by(&self, scope: &mut Scope, query_node: ast::Node<'_>) {
        let Some(select) = query_node.as_select() else {
            return;
        };

        let Some(group_by) = &select.group_by else {
            return;
        };

        // Extract column references from GROUP BY items
        for item in &group_by.item.items.items {
            if let ast::GroupByItem::Expr(expr) = &item.item {
                // Only extract simple column references
                if let ast::Expr::Name(_name) = &expr.item {
                    // Get the column name and qualifier
                    let col_name = self.projection_name(expr);
                    let qualifier = self.get_qualifier_from_expr(expr);
                    let origin = self.column_origin(scope, expr);

                    let id = ColumnId(scope.grouped.len() as u32);
                    scope.grouped.push(BoundColumn {
                        id,
                        name: col_name,
                        origin,
                        qualifier,
                    });
                }
            }
        }
    }

    fn gather_order_by(&self, scope: &mut Scope, query_node: ast::Node<'_>) {
        let ast::Node::Query(query) = query_node else {
            return;
        };

        let Some(tail) = &query.item.tail else {
            return;
        };

        let Some(order_by) = &tail.item.order_by else {
            return;
        };

        // Extract column references from ORDER BY items
        for item in &order_by.item.items.items {
            let expr = &item.item.expr;
            // Only extract simple column references
            if let ast::Expr::Name(_name) = &expr.item {
                // Get the column name and qualifier
                let col_name = self.projection_name(expr);
                let qualifier = self.get_qualifier_from_expr(expr);
                let origin = self.column_origin(scope, expr);

                let id = ColumnId(scope.ordered.len() as u32);
                scope.ordered.push(BoundColumn {
                    id,
                    name: col_name,
                    origin,
                    qualifier,
                });
            }
        }
    }

    // Helper methods to extract strings and paths from AST nodes
    fn span_string(&self, span: &ast::SpannedStr) -> String {
        span.as_str(self.text).to_string()
    }

    fn span_str(&self, span: &ast::SpannedStr) -> &str {
        span.as_str(self.text)
    }

    fn name_path(&self, name: &Loc<ast::QualifiedName>) -> NamePath {
        name.item
            .parts
            .items
            .iter()
            .map(|part| self.span_str(&part.span))
            .into()
    }

    fn projection_name(&self, expr: &Loc<ast::Expr>) -> String {
        match &expr.item {
            ast::Expr::Name(name) => self.get_name(name),
            _ => self.span_string(&expr.span),
        }
    }

    fn get_qualifier_from_expr(&self, expr: &Loc<ast::Expr>) -> Qualifier {
        match &expr.item {
            ast::Expr::Name(name) => self.get_qualifier(name),
            _ => Qualifier::default(),
        }
    }

    fn get_qualifier(&self, name: &QualifiedName) -> Qualifier {
        Qualifier::from(
            name.parts
                .items
                .iter()
                .take(name.parts.items.len().saturating_sub(1))
                .map(|part| self.span_string(&part.span))
                .collect::<Vec<_>>(),
        )
    }

    fn get_name(&self, name: &QualifiedName) -> String {
        name.parts
            .items
            .last()
            .map(|part| self.span_string(&part.span))
            .unwrap_or_default()
    }

    fn column_origin(&self, scope: &Scope, expr: &Loc<ast::Expr>) -> Origin {
        if let ast::Expr::Literal(literal) = &expr.item {
            match &literal.item {
                ast::Literal::Number(_) => return Origin::Constant(Literal::Number),
                &ast::Literal::Boolean(_) => return Origin::Constant(Literal::Boolean),
                &ast::Literal::String(_) => return Origin::Constant(Literal::String),
                &ast::Literal::Null => return Origin::Constant(Literal::Null),
                _ => return Origin::UnresolvedIdent(NamePath(vec![])),
            };
        };

        let ast::Expr::Name(name) = &expr.item else {
            return Origin::UnresolvedIdent(NamePath(vec![]));
        };

        let qualifier = self.get_qualifier(name);
        let relation = qualifier
            .table
            .as_ref()
            .and_then(|table| scope.by_name.get(table).copied())
            .or_else(|| {
                (scope.relations.len() == 1)
                    .then(|| scope.relations.keys().next().copied())
                    .flatten()
            });

        // Check for star expression
        if let Some(last) = name.parts.items.last()
            && matches!(last.item, ast::NamePart::Star)
        {
            return Origin::Star { relation };
        }

        if let Some(relation) = relation {
            return Origin::BaseColumn {
                relation: relation,
                name: self.get_name(name),
            };
        }

        Origin::UnresolvedIdent(self.name_path(name))
    }
}

pub fn build_scope<'txt>(text: &'txt str, position: usize, ast: ast::Node<'txt>) -> Scope {
    ScopeBuilder::new(text, position, ast).build()
}

#[cfg(test)]
mod tests {
    use crate::{
        parse::Parser,
        test_util::{ansi_tokens, with_caret_cursor},
    };

    use super::*;

    #[test]
    fn select() {
        let s = ansi_detect_scope("SELECT name, ^");
        s.assert_projection(0, "name", Origin::UnresolvedIdent(to_name_path("name")));
        let s = ansi_detect_scope("SELECT a.c, b^");
        s.assert_projection(0, "c", Origin::UnresolvedIdent(to_name_path("a.c")));
        s.assert_projection(1, "b", Origin::UnresolvedIdent(to_name_path("b")));
    }

    #[test]
    fn select_from() {
        let s = ansi_detect_scope("SELECT * FROM users^");
        s.assert_base_relation("users", "users");
        s.assert_projection(
            0,
            "*",
            Origin::Star {
                relation: Some(s.relation("users").unwrap()),
            },
        );
    }

    #[test]
    fn from_alias() {
        let s = ansi_detect_scope("SELECT * FROM users u^");
        s.assert_base_relation("u", "users");
        s.assert_projection(
            0,
            "*",
            Origin::Star {
                relation: Some(s.relation("u").unwrap()),
            },
        );
    }

    #[test]
    fn from_schema_qualified() {
        let s = ansi_detect_scope("SELECT * FROM public.users^");
        s.assert_base_relation("users", "public.users");
    }

    #[test]
    fn from_multiple_sources() {
        let s = ansi_detect_scope("SELECT * FROM users, orders^");
        s.assert_base_relation("users", "users");
        s.assert_base_relation("orders", "orders");
    }

    #[test]
    fn from_subquery() {
        let s = ansi_detect_scope("SELECT * FROM (SELECT * FROM users) u^");
        assert!(matches!(s.binding("u").kind, RelationKind::Subquery(_)));
        s.assert_projection(
            0,
            "*",
            Origin::Star {
                relation: Some(s.relation("u").unwrap()),
            },
        );
    }

    // Test utilities
    fn to_name_path(name: &str) -> NamePath {
        NamePath(name.split('.').map(|s| s.to_string()).collect())
    }

    fn ansi_detect_scope(sql: &str) -> Scope {
        let (text, pos) = with_caret_cursor(sql);
        let tokens = ansi_tokens(&text);
        let statement = Parser::new(&tokens).parse_statement().unwrap();
        build_scope(&text, pos, ast::Node::Statement(&statement))
    }

    // Helper trait to make test assertions more concise
    trait ScopeTestExt {
        fn binding(&self, name: &str) -> &RelationBinding;
        fn assert_base_relation(&self, name: &str, expected: &str);
        fn assert_projection(&self, idx: usize, expected_name: &str, expected_origin: Origin);
    }

    impl ScopeTestExt for Scope {
        fn binding(&self, name: &str) -> &RelationBinding {
            let id = self.by_name.get(name).unwrap();
            self.relations.get(id).unwrap()
        }

        fn assert_base_relation(&self, name: &str, expected: &str) {
            assert_eq!(
                self.binding(name).kind,
                RelationKind::Base(to_name_path(expected))
            );
        }

        fn assert_projection(&self, idx: usize, expected_name: &str, expected_origin: Origin) {
            assert_eq!(self.projected[idx].name, expected_name);
            assert_eq!(self.projected[idx].origin, expected_origin);
        }
    }
}
