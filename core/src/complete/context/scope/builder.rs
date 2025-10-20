use crate::ast::{self};
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

            let qualifier = self.projection_qualifier(&item.expr);

            scope.insert_column(name, self.column_origin(scope, &item.expr), qualifier, None);
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
                    let qualifier = self.projection_qualifier(expr);
                    let origin = self.column_origin(scope, expr);

                    let id = ColumnId(scope.grouped.len() as u32);
                    scope.grouped.push(BoundColumn {
                        id,
                        name: col_name,
                        origin,
                        qualifier,
                        ty: None,
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
                let qualifier = self.projection_qualifier(expr);
                let origin = self.column_origin(scope, expr);

                let id = ColumnId(scope.ordered.len() as u32);
                scope.ordered.push(BoundColumn {
                    id,
                    name: col_name,
                    origin,
                    qualifier,
                    ty: None,
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
            ast::Expr::Name(name) => name
                .parts
                .items
                .last()
                .map(|part| self.span_string(&part.span))
                .unwrap_or_default(),
            _ => self.span_string(&expr.span),
        }
    }

    fn projection_qualifier(&self, expr: &Loc<ast::Expr>) -> Option<String> {
        match &expr.item {
            ast::Expr::Name(name) if name.parts.items.len() >= 2 => {
                // For qualified names like "users.name", return "users"
                // Only take the first part as the qualifier
                Some(self.span_string(&name.parts.items[0].span))
            }
            _ => None,
        }
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

        // Check for star expression
        if let Some(last) = name.parts.items.last()
            && matches!(last.item, ast::NamePart::Star)
        {
            let relation = if name.parts.items.len() > 1 {
                let col = &name.parts.items[name.parts.items.len().saturating_sub(1)];
                scope.relation(self.span_str(&col.span))
            } else {
                // Unqualified star: *
                (scope.relations.len() == 1)
                    .then(|| scope.relations.keys().next().copied())
                    .flatten()
            };
            return Origin::Star { relation };
        }

        // Try to resolve column reference to a specific table
        let parts: Vec<&str> = name
            .parts
            .items
            .iter()
            .map(|p| self.span_str(&p.span))
            .collect();

        match parts.len() {
            1 => {
                // Unqualified column name - try to find which relation it belongs to
                let col_name = parts[0];

                // If there's only one relation in scope, assume the column comes from it
                if scope.relations.len() == 1
                    && let Some((&rel_id, _)) = scope.relations.iter().next()
                {
                    return Origin::BaseColumn {
                        relation: rel_id,
                        name: col_name.to_string(),
                    };
                }

                // Fall back to unresolved
                Origin::UnresolvedIdent(self.name_path(name))
            }
            2 => {
                // Qualified column name: table.column or schema.column
                let qualifier = parts[0];
                let col_name = parts[1];

                // Try to find the relation by name
                if let Some(rel_id) = scope.relation(qualifier) {
                    return Origin::BaseColumn {
                        relation: rel_id,
                        name: col_name.to_string(),
                    };
                }

                // Fall back to unresolved
                Origin::UnresolvedIdent(self.name_path(name))
            }
            _ => {
                // More complex qualified names - keep as unresolved for now
                Origin::UnresolvedIdent(self.name_path(name))
            }
        }
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
