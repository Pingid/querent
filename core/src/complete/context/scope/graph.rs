use std::collections::HashMap;

use crate::ast;
use crate::complete::context::ContextBuildParams;
use crate::schema;
use crate::span::Loc;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScopeGraph<'a> {
    /// By alias or relation name -> RelationId (for quick lookup).
    pub by_name: HashMap<&'a str, BindingId>,
    /// All bound relations.
    pub bindings: HashMap<BindingId, RelationBinding<'a>>,
    /// Output columns of the innermost SELECT list (post-aliasing).
    pub projected: Vec<BoundColumn<'a>>,
    /// Columns referenced in the GROUP BY clause.
    pub grouped: Vec<BoundColumn<'a>>,
    /// Columns referenced in the ORDER BY clause.
    pub ordered: Vec<BoundColumn<'a>>,
}

/// Accessor methods for [`Relations`].
impl<'a> ScopeGraph<'a> {
    pub fn relation(&self, name: &str) -> Option<BindingId> {
        self.by_name.get(name).copied()
    }
    pub fn build(text: &'a str, position: usize, ast: ast::Node<'_>) -> ScopeGraph<'a> {
        ScopeGraphBuilder::new(text, position, ast).build()
    }
}

impl<'a> From<&ContextBuildParams<'a>> for ScopeGraph<'a> {
    fn from(params: &ContextBuildParams<'a>) -> Self {
        ScopeGraphBuilder::new(
            params.text,
            params.cursor,
            ast::Node::Statement(&params.stmt),
        )
        .build()
    }
}

/// Builder methods for [`Relations`].
impl<'a> ScopeGraph<'a> {
    pub fn insert_relation(&mut self, kind: BindingKind<'a>, alias: Option<&'a str>) -> BindingId {
        let id = BindingId(self.bindings.len() as u32);

        self.bindings.insert(
            id,
            RelationBinding {
                id,
                kind,
                alias,
                columns: Vec::new(),
            },
        );

        if let Some(a) = alias {
            self.by_name.insert(a, id);
        }

        id
    }

    pub fn insert_column(
        &mut self, name: &'a str, origin: Origin<'a>, qualifier: Qualifier<'a>,
    ) -> ColumnId {
        let id = ColumnId(self.projected.len() as u32);
        self.projected.push(BoundColumn {
            id,
            name,
            origin,
            qualifier,
        });
        id
    }
}

/// Unique handles so aliases & nested bindings are unambiguous.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BindingId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnId(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub struct RelationBinding<'a> {
    pub id: BindingId,
    pub kind: BindingKind<'a>,
    pub alias: Option<&'a str>,
    pub columns: Vec<BoundColumn<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BindingKind<'a> {
    Base(NamePath<'a>),            // e.g. schema.table
    Cte(Box<ScopeGraph<'a>>),      // CTE with its relations
    Subquery(Box<ScopeGraph<'a>>), // keep for lazy expansion or rebuild
}

/// Column visible in a relation binding or projection output.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundColumn<'a> {
    pub id: ColumnId,
    pub name: &'a str,      // visible name (incl. alias)
    pub origin: Origin<'a>, // lineage
    pub qualifier: Qualifier<'a>,
}

/// Where a column ultimately comes from; enables lineage & re-type.
#[derive(Debug, Clone, PartialEq)]
pub enum Origin<'a> {
    UnresolvedIdent(NamePath<'a>),
    Constant(Literal),
    /// Directly from a base table column.
    BaseColumn {
        relation: BindingId,
        name: &'a str, // base column name
    },
    /// From another bound column (used for SELECT passthrough, CTEs).
    // FromColumn(ColumnId),
    // /// Computed expression. Keep input lineage for “deep” tracking.
    // DerivedExpr {
    //     expr: Loc<ast::Expr>,
    //     inputs: Vec<Origin>,
    // },
    /// Wildcard expansion marker (resolved to concrete columns upstream).
    Star {
        relation: Option<BindingId>, // None => unqualified *
    },
    FunctionCall {
        name: &'a str,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number,
    Float,
    String,
    Boolean,
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamePath<'a>(pub Vec<&'a str>);

impl<'a> NamePath<'a> {
    pub fn to_string(&self) -> String {
        self.0.join(".")
    }

    pub fn from_qualified_name(text: &'a str, name: &ast::QualifiedName) -> NamePath<'a> {
        Self(
            name.parts
                .items
                .iter()
                .map(|part| part.span.as_str(text))
                .collect(),
        )
    }
}

impl<'a> From<Vec<&'a str>> for NamePath<'a> {
    fn from(path: Vec<&'a str>) -> Self {
        NamePath(path)
    }
}

/// The qualifier used in the source (e.g., "users" in "users.name")
#[derive(Debug, Default, Clone, PartialEq, Hash, Eq)]
pub struct Qualifier<'a> {
    pub schema: Option<&'a str>,
    pub table: Option<&'a str>,
}

impl Qualifier<'_> {
    pub fn is_empty(&self) -> bool {
        self.schema.is_none() && self.table.is_none()
    }
}

impl<'a> From<Vec<&'a str>> for Qualifier<'a> {
    fn from(qualifier: Vec<&'a str>) -> Self {
        let mut q = qualifier;
        let table = q.pop();
        let schema = q.pop();
        Self { schema, table }
    }
}

impl<'a> From<&'a schema::Column> for Qualifier<'a> {
    fn from(col: &'a schema::Column) -> Self {
        Self {
            schema: col.schema_name.as_ref().map(|x| x.as_str()),
            table: col.table_name.as_ref().map(|x| x.as_str()),
        }
    }
}

impl<'a> From<&Vec<&'a str>> for Qualifier<'a> {
    fn from(qualifier: &Vec<&'a str>) -> Self {
        Qualifier::from(qualifier.clone())
    }
}

impl ToString for Qualifier<'_> {
    fn to_string(&self) -> String {
        match (self.schema.as_ref(), self.table.as_ref()) {
            (Some(schema), Some(table)) => format!("{}.{}", schema, table),
            (Some(schema), None) => schema.to_string(),
            (None, Some(table)) => table.to_string(),
            (None, None) => String::new(),
        }
    }
}

pub struct ScopeGraphBuilder<'txt, 'ast> {
    pub text: &'txt str,
    pub position: usize,
    pub ast: ast::Node<'ast>,
}

impl<'txt, 'ast> ScopeGraphBuilder<'txt, 'ast> {
    pub fn new(text: &'txt str, position: usize, ast: ast::Node<'ast>) -> Self {
        Self {
            text,
            position,
            ast,
        }
    }

    pub fn build(&self) -> ScopeGraph<'txt> {
        let query = self.ast.find_rev(
            |node| matches!(node, ast::Node::Query(q) if q.span.contains_inclusive(self.position) || (q.span.end <= self.position && self.text[q.span.end..self.position].chars().all(|c| c.is_whitespace()))),
        );

        let Some(query_node) = query else {
            return ScopeGraph::default();
        };
        self.gather_relations(query_node)
    }

    fn gather_relations(&self, query_node: impl Into<ast::Node<'ast>>) -> ScopeGraph<'txt> {
        let mut relations = ScopeGraph::default();
        let query_node = query_node.into();
        self.gather_ctes(&mut relations, query_node);
        self.gather_from(&mut relations, query_node);
        self.gather_projections(&mut relations, query_node);
        self.gather_group_by(&mut relations, query_node);
        self.gather_order_by(&mut relations, query_node);
        relations
    }

    fn gather_ctes(&self, scope: &mut ScopeGraph<'txt>, query_node: ast::Node<'ast>) {
        let ast::Node::Query(query) = query_node else {
            return;
        };

        let Some(with) = &query.item.with else {
            return;
        };

        // Process each CTE
        for cte in &with.item.ctes {
            let name = self.span_str(&cte.item.name);
            let cte_scope = self.gather_relations(&*cte.item.query);
            scope.insert_relation(BindingKind::<'txt>::Cte(Box::new(cte_scope)), Some(name));
        }
    }

    fn gather_from(&self, scope: &mut ScopeGraph<'txt>, query_node: ast::Node<'ast>) {
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

    fn gather_table_ref(&self, scope: &mut ScopeGraph<'txt>, table_ref: &Loc<ast::TableRef>) {
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

    fn gather_table_factor(&self, scope: &mut ScopeGraph<'txt>, factor: &Loc<ast::TableFactor>) {
        match &factor.item {
            ast::TableFactor::Named(n) => {
                let path = self.name_path(&n.item.name);
                let alias = n.item.alias.as_ref().map(|a| self.span_str(&a.span));

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
                    scope.insert_relation(BindingKind::Base(path), alias);
                }
            }
            ast::TableFactor::Subquery(n) => {
                let alias = n.item.alias.as_ref().map(|a| self.span_str(&a.span));
                let inner_scope: ScopeGraph<'txt> =
                    self.gather_relations(ast::Node::Query(&n.item.query));
                let kind = BindingKind::<'txt>::Subquery(Box::new(inner_scope));
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

    fn gather_projections(&self, scope: &mut ScopeGraph<'txt>, query_node: ast::Node<'ast>) {
        let Some(select) = query_node.as_select() else {
            return;
        };

        for item in select.projection.items() {
            let name = item
                .alias
                .as_ref()
                .map(|a| self.span_str(&a.span))
                .unwrap_or_else(|| self.projection_name(&item.expr));

            let qualifier = self.get_qualifier_from_expr(&item.expr);
            scope.insert_column(name, self.column_origin(scope, &item.expr), qualifier);
        }
    }

    fn gather_group_by(&self, scope: &mut ScopeGraph<'txt>, query_node: ast::Node<'ast>) {
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

    fn gather_order_by(&self, scope: &mut ScopeGraph<'txt>, query_node: ast::Node<'ast>) {
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

    fn span_str(&self, span: &ast::SpannedStr) -> &'txt str {
        span.as_str(self.text)
    }

    fn name_path(&self, name: &Loc<ast::QualifiedName>) -> NamePath<'txt> {
        name.item
            .parts
            .items
            .iter()
            .map(|part| self.span_str(&part.span))
            .collect::<Vec<_>>()
            .into()
    }

    fn projection_name(&self, expr: &Loc<ast::Expr>) -> &'txt str {
        match &expr.item {
            ast::Expr::Name(name) => self.get_name(name),
            _ => self.span_str(&expr.span),
        }
    }

    fn get_qualifier_from_expr(&self, expr: &Loc<ast::Expr>) -> Qualifier<'txt> {
        match &expr.item {
            ast::Expr::Name(name) => self.get_qualifier(name),
            _ => Qualifier::default(),
        }
    }

    fn get_qualifier(&self, name: &ast::QualifiedName) -> Qualifier<'txt> {
        Qualifier::from(
            name.parts
                .items
                .iter()
                .take(name.parts.items.len().saturating_sub(1))
                .map(|part| self.span_str(&part.span))
                .collect::<Vec<_>>(),
        )
    }

    fn get_name(&self, name: &ast::QualifiedName) -> &'txt str {
        name.parts
            .items
            .last()
            .map(|part| self.span_str(&part.span))
            .unwrap()
    }

    fn column_origin(&self, scope: &ScopeGraph<'txt>, expr: &Loc<ast::Expr>) -> Origin<'txt> {
        if let ast::Expr::Literal(literal) = &expr.item {
            match &literal.item {
                ast::Literal::Number(_) => return Origin::Constant(Literal::Number),
                &ast::Literal::Boolean(_) => return Origin::Constant(Literal::Boolean),
                &ast::Literal::String(_) => return Origin::Constant(Literal::String),
                &ast::Literal::Null => return Origin::Constant(Literal::Null),
                _ => return Origin::UnresolvedIdent(NamePath(vec![])),
            };
        };

        if let ast::Expr::FunctionCall(function_call) = &expr.item {
            return Origin::FunctionCall {
                name: self.span_str(&function_call.name.span),
            };
        }

        let ast::Expr::Name(name) = &expr.item else {
            return Origin::UnresolvedIdent(NamePath(vec![]));
        };

        let qualifier = self.get_qualifier(name);
        let relation = qualifier
            .table
            .as_ref()
            .and_then(|table| scope.by_name.get(table).copied())
            .or_else(|| {
                (scope.bindings.len() == 1)
                    .then(|| scope.bindings.keys().next().copied())
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
                relation,
                name: self.get_name(name),
            };
        }

        Origin::UnresolvedIdent(self.name_path(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::Parser;
    use crate::test_util::ansi_tokens;
    use crate::test_util::get_leaky_static_caret_cursor;

    #[test]
    fn select() {
        let s = RelationsFixture::new("SELECT name, ^");
        s.assert_projection(0, "name", Origin::UnresolvedIdent(to_name_path("name")));
        let s = RelationsFixture::new("SELECT a.c, b^");
        s.assert_projection(0, "c", Origin::UnresolvedIdent(to_name_path("a.c")));
        s.assert_projection(1, "b", Origin::UnresolvedIdent(to_name_path("b")));
    }

    #[test]
    fn from_alias() {
        let s = RelationsFixture::new("SELECT * FROM users u^");
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
    fn from_subquery() {
        let s = RelationsFixture::new("SELECT * FROM (SELECT * FROM users) u^");
        assert!(matches!(s.binding("u").kind, BindingKind::Subquery(_)));
        s.assert_projection(
            0,
            "*",
            Origin::Star {
                relation: Some(s.relation("u").unwrap()),
            },
        );
    }

    struct RelationsFixture {
        relations: ScopeGraph<'static>,
    }

    impl RelationsFixture {
        fn new(input: &str) -> Self {
            let (text, pos) = get_leaky_static_caret_cursor(input);
            let tokens = ansi_tokens(text);
            let statement = Parser::new(&tokens).parse_statement().unwrap();
            let relations =
                ScopeGraphBuilder::new(text, pos, ast::Node::Statement(&statement)).build();
            Self { relations }
        }
    }

    // Test utilities
    fn to_name_path<'a>(name: &'a str) -> NamePath<'a> {
        NamePath(name.split('.').map(|s| s).collect())
    }

    impl RelationsFixture {
        fn relation(&self, name: &str) -> Option<BindingId> {
            self.relations.relation(name)
        }

        fn binding(&self, name: &str) -> &RelationBinding<'static> {
            let id = self.relations.by_name.get(name).unwrap();
            self.relations.bindings.get(id).unwrap()
        }

        fn assert_base_relation(&self, name: &str, expected: &str) {
            assert_eq!(
                self.binding(name).kind,
                BindingKind::Base(to_name_path(expected))
            );
        }

        fn assert_projection(&self, idx: usize, expected_name: &str, expected_origin: Origin) {
            assert_eq!(self.relations.projected[idx].name, expected_name);
            assert_eq!(self.relations.projected[idx].origin, expected_origin);
        }
    }
}
