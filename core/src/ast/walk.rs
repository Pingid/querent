use std::ops::ControlFlow;

use crate::ast;
use crate::span::Loc;

/// A reference to any node in the AST.
///
/// This enum provides a unified type for traversing the entire SQL AST
/// hierarchy. Each variant holds a borrowed reference to a specific node type,
/// allowing zero-copy tree traversal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Node<'a> {
    // Top-level
    Statement(&'a Loc<ast::Statement>),

    // Query structure
    Query(&'a Loc<ast::Query>),
    With(&'a Loc<ast::With>),
    Cte(&'a Loc<ast::Cte>),
    SetOpTerm(&'a Loc<ast::SetOpTerm>),
    QueryPrimary(&'a Loc<ast::QueryPrimary>),

    // SELECT statement
    Select(&'a Loc<ast::Select>),
    ProjectionItem(&'a Loc<ast::ProjectionItem>),

    // FROM clause
    From(&'a Loc<ast::From>),
    TableRef(&'a Loc<ast::TableRef>),
    TableFactor(&'a Loc<ast::TableFactor>),
    NamedTableFactor(&'a Loc<ast::NamedTableFactor>),
    FunctionTableFactor(&'a Loc<ast::FunctionTableFactor>),
    SubqueryTableFactor(&'a Loc<ast::SubqueryTableFactor>),
    Join(&'a Loc<ast::Join>),
    JoinConstraint(&'a Loc<ast::JoinConstraint>),

    // GROUP BY clause
    GroupBy(&'a Loc<ast::GroupBy>),
    GroupByItem(&'a Loc<ast::GroupByItem>),
    GroupingSet(&'a Loc<ast::GroupingSet>),

    // WINDOW clause
    Window(&'a Loc<ast::Window>),
    WindowDef(&'a Loc<ast::WindowDef>),
    WindowSpec(&'a Loc<ast::WindowSpec>),
    WindowFrame(&'a Loc<ast::WindowFrame>),
    FrameBound(&'a Loc<ast::FrameBound>),

    // ORDER BY / LIMIT / OFFSET
    QuerySuffix(&'a Loc<ast::QuerySuffix>),
    OrderBy(&'a Loc<ast::OrderBy>),
    OrderByItem(&'a Loc<ast::OrderByItem>),
    Limit(&'a Loc<ast::Limit>),
    Offset(&'a Loc<ast::Offset>),

    // VALUES statement
    Values(&'a Loc<ast::Values>),

    // Expressions
    Expr(&'a Loc<ast::Expr>),
    BinaryExpr(&'a Loc<ast::BinaryExpr>),
    UnaryExpr(&'a Loc<ast::UnaryExpr>),
    ParenExpr(&'a Loc<ast::ParenExpr>),
    IsNullExpr(&'a Loc<ast::IsNullExpr>),
    BetweenExpr(&'a Loc<ast::BetweenExpr>),
    LikeExpr(&'a Loc<ast::LikeExpr>),
    ILikeExpr(&'a Loc<ast::ILikeExpr>),
    SimilarExpr(&'a Loc<ast::SimilarExpr>),
    FunctionCallExpr(&'a Loc<ast::FunctionCallExpr>),
    QuantifiedExpr(&'a Loc<ast::QuantifiedExpr>),
    CaseExpr(&'a Loc<ast::CaseExpr>),
    InExpr(&'a Loc<ast::InExpr>),
    OverExpr(&'a Loc<ast::OverExpr>),

    // Literals
    Literal(&'a Loc<ast::Literal>),

    // Names and identifiers
    QualifiedName(&'a Loc<ast::QualifiedName>),
    SpannedStr(&'a Loc<ast::SpannedStr>),
}

// Implement From<&'a Loc<ast::$n>> for Node<'a>
macro_rules! impl_from {
    ($n:ident) => {
        impl<'a> From<&'a Loc<ast::$n>> for Node<'a> {
            fn from(t: &'a Loc<ast::$n>) -> Self {
                Node::$n(t)
            }
        }
    };
    ($($n:ident),+ $(,)?) => {
        $( impl_from!($n); )+
    };
}

#[rustfmt::skip]
impl_from!(Statement, Query, With, Cte, SetOpTerm, QueryPrimary, Select, ProjectionItem, From, TableRef, TableFactor, NamedTableFactor, FunctionTableFactor, SubqueryTableFactor, Join, JoinConstraint, GroupBy, GroupByItem, GroupingSet, Window, WindowDef, WindowSpec, WindowFrame, FrameBound, QuerySuffix, OrderBy, OrderByItem, Limit, Offset, Values, Expr, BinaryExpr, UnaryExpr, ParenExpr, IsNullExpr, BetweenExpr, LikeExpr, ILikeExpr, SimilarExpr, FunctionCallExpr, QuantifiedExpr, CaseExpr, InExpr, OverExpr, Literal, QualifiedName, SpannedStr);

/// Events emitted during AST traversal.
#[derive(Debug, Clone, Copy)]
pub enum AstEvent<'a> {
    /// Emitted when entering a node (before visiting its children).
    Enter(Node<'a>),
    /// Emitted when exiting a node (after visiting all its children).
    Exit(Node<'a>),
}

/// AST traversal operations.
impl<'a> Node<'a> {
    /// Performs a depth-first traversal of the AST, calling the visitor
    /// function for each node with both `Enter` and `Exit` events.
    pub fn visit<T, F>(&self, visitor: &mut F) -> ControlFlow<T>
    where F: FnMut(AstEvent<'a>) -> ControlFlow<T> {
        fn visit<'a, T, F>(node: Node<'a>, visitor: &mut F) -> ControlFlow<T>
        where F: FnMut(AstEvent<'a>) -> ControlFlow<T> {
            visitor(AstEvent::Enter(node))?;
            node.visit(visitor)?;
            visitor(AstEvent::Exit(node))
        }

        match self {
            // Top-level
            Node::Statement(stmt) => match &stmt.item {
                ast::Statement::Query(n) => visit(Node::Query(n), visitor),
                ast::Statement::Partial(_) => ControlFlow::Continue(()),
            },

            // Query structure
            Node::Query(n) => {
                if let Some(with) = &n.item.with {
                    visit(Node::With(with), visitor)?;
                }
                if let Some(body) = &n.item.body {
                    visit(Node::QueryPrimary(&body.left), visitor)?;
                    for chain in &body.set_ops {
                        visit(Node::SetOpTerm(chain), visitor)?;
                    }
                }
                if let Some(tail) = &n.item.tail {
                    visit(Node::QuerySuffix(tail), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::With(n) => {
                for cte in &n.ctes {
                    visit(Node::Cte(cte), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::Cte(n) => {
                if let Some(columns) = &n.columns {
                    for column in &columns.items {
                        visit(Node::SpannedStr(column), visitor)?;
                    }
                }
                visit(Node::Query(&n.query), visitor)
            }
            Node::SetOpTerm(n) => visit(Node::QueryPrimary(&n.right), visitor),
            Node::QueryPrimary(query) => match &query.item {
                ast::QueryPrimary::Select(n) => visit(Node::Select(n), visitor),
                ast::QueryPrimary::Values(n) => visit(Node::Values(n), visitor),
                ast::QueryPrimary::Parenthesized(n) => visit(Node::Query(n), visitor),
            },

            // SELECT statement
            Node::Select(stmt) => {
                for item in &stmt.projection.items {
                    visit(Node::ProjectionItem(item), visitor)?;
                }
                if let Some(from) = &stmt.from {
                    visit(Node::From(from), visitor)?;
                }
                if let Some(where_clause) = &stmt.where_clause {
                    visit(Node::Expr(where_clause), visitor)?;
                }
                if let Some(group_by) = &stmt.group_by {
                    visit(Node::GroupBy(group_by), visitor)?;
                }
                if let Some(having) = &stmt.having {
                    visit(Node::Expr(having), visitor)?;
                }
                if let Some(window) = &stmt.window {
                    visit(Node::Window(window), visitor)?;
                }
                if let Some(qualify) = &stmt.qualify {
                    visit(Node::Expr(qualify), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::ProjectionItem(n) => visit(Node::Expr(&n.expr), visitor),

            // FROM clause
            Node::From(n) => {
                for source in &n.sources.items {
                    visit(Node::TableRef(source), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::TableRef(n) => match &n.item {
                ast::TableRef::Factor(f) => visit(Node::TableFactor(f), visitor),
                ast::TableRef::Join(j) => visit(Node::Join(j), visitor),
            },
            Node::TableFactor(p) => match &p.item {
                ast::TableFactor::Named(n) => visit(Node::NamedTableFactor(n), visitor),
                ast::TableFactor::Function(n) => visit(Node::FunctionTableFactor(n), visitor),
                ast::TableFactor::Subquery(n) => visit(Node::SubqueryTableFactor(n), visitor),
                ast::TableFactor::Parenthesized(n) => visit(Node::TableRef(n), visitor),
            },
            Node::NamedTableFactor(n) => visit(Node::QualifiedName(&n.name), visitor),
            Node::FunctionTableFactor(n) => {
                for arg in &n.args.items {
                    visit(Node::Expr(arg), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::SubqueryTableFactor(n) => visit(Node::Query(&n.query), visitor),
            Node::Join(n) => {
                visit(Node::TableRef(&n.left), visitor)?;
                visit(Node::TableRef(&n.right), visitor)?;
                if let Some(constraint) = &n.constraint {
                    visit(Node::JoinConstraint(constraint), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::JoinConstraint(n) => match &n.item {
                ast::JoinConstraint::On(e) => visit(Node::Expr(e), visitor),
                ast::JoinConstraint::Using(es) => {
                    for e in &es.items {
                        visit(Node::SpannedStr(e), visitor)?;
                    }
                    ControlFlow::Continue(())
                }
            },

            // GROUP BY clause
            Node::GroupBy(n) => {
                for item in &n.items.items {
                    visit(Node::GroupByItem(item), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::GroupByItem(n) => match &n.item {
                ast::GroupByItem::Expr(e) => visit(Node::Expr(e), visitor),
                ast::GroupByItem::Rollup(es) | ast::GroupByItem::Cube(es) => {
                    for e in es {
                        visit(Node::Expr(e), visitor)?;
                    }
                    ControlFlow::Continue(())
                }
                ast::GroupByItem::GroupingSets(sets) => {
                    for gs in sets {
                        visit(Node::GroupingSet(gs), visitor)?;
                    }
                    ControlFlow::Continue(())
                }
            },
            Node::GroupingSet(n) => match &n.item {
                ast::GroupingSet::Expr(e) => visit(Node::Expr(e), visitor),
                ast::GroupingSet::Exprs(es) => {
                    for e in es {
                        visit(Node::Expr(e), visitor)?;
                    }
                    ControlFlow::Continue(())
                }
            },

            // WINDOW clause
            Node::Window(n) => {
                for window in &n.windows {
                    visit(Node::WindowDef(window), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::WindowDef(n) => visit(Node::WindowSpec(&n.spec), visitor),
            Node::WindowSpec(n) => {
                if let Some(pb) = &n.partition_by {
                    for item in &pb.items {
                        visit(Node::Expr(item), visitor)?;
                    }
                }
                if let Some(ob) = &n.order_by {
                    visit(Node::OrderBy(ob), visitor)?;
                }
                if let Some(frame) = &n.frame {
                    visit(Node::WindowFrame(frame), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::WindowFrame(n) => {
                visit(Node::FrameBound(&n.start), visitor)?;
                if let Some(end) = &n.end {
                    visit(Node::FrameBound(end), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::FrameBound(n) => match &n.item {
                ast::FrameBound::Preceding(e) | ast::FrameBound::Following(e) => {
                    visit(Node::Expr(e), visitor)
                }
                _ => ControlFlow::Continue(()),
            },

            // ORDER BY / LIMIT / OFFSET
            Node::QuerySuffix(n) => {
                if let Some(order_by) = &n.order_by {
                    visit(Node::OrderBy(order_by), visitor)?;
                }
                if let Some(limit) = &n.limit {
                    visit(Node::Limit(limit), visitor)?;
                }
                if let Some(offset) = &n.offset {
                    visit(Node::Offset(offset), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::OrderBy(n) => {
                for item in &n.items.items {
                    visit(Node::OrderByItem(item), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::OrderByItem(n) => visit(Node::Expr(&n.expr), visitor),
            Node::Limit(n) => visit(Node::Expr(&n.count), visitor),
            Node::Offset(n) => visit(Node::Expr(&n.count), visitor),

            // VALUES statement
            Node::Values(stmt) => {
                for row in &stmt.rows {
                    for item in &row.items {
                        visit(Node::Expr(item), visitor)?;
                    }
                }
                ControlFlow::Continue(())
            }

            // Expressions
            Node::Expr(e) => match &e.item {
                ast::Expr::Name(n) => visit(Node::QualifiedName(n), visitor),
                ast::Expr::Literal(l) => visit(Node::Literal(l), visitor),
                ast::Expr::Binary(b) => visit(Node::BinaryExpr(b), visitor),
                ast::Expr::Unary(u) => visit(Node::UnaryExpr(u), visitor),
                ast::Expr::Paren(p) => visit(Node::ParenExpr(p), visitor),
                ast::Expr::Subquery(q) => visit(Node::Query(q), visitor),
                ast::Expr::IsNull(isn) => visit(Node::IsNullExpr(isn), visitor),
                ast::Expr::Between(b) => visit(Node::BetweenExpr(b), visitor),
                ast::Expr::Like(l) => visit(Node::LikeExpr(l), visitor),
                ast::Expr::ILike(il) => visit(Node::ILikeExpr(il), visitor),
                ast::Expr::Similar(s) => visit(Node::SimilarExpr(s), visitor),
                ast::Expr::FunctionCall(fc) => visit(Node::FunctionCallExpr(fc), visitor),
                ast::Expr::Array(items) => {
                    for item in &items.items {
                        visit(Node::Expr(item), visitor)?;
                    }
                    ControlFlow::Continue(())
                }
                ast::Expr::Quantified(q) => visit(Node::QuantifiedExpr(q), visitor),
                ast::Expr::Case(c) => visit(Node::CaseExpr(c), visitor),
                ast::Expr::In(i) => visit(Node::InExpr(i), visitor),
                ast::Expr::Over(o) => visit(Node::OverExpr(o), visitor),
                ast::Expr::Exists(q) => visit(Node::Query(q), visitor),
                ast::Expr::Empty => ControlFlow::Continue(()),
            },
            Node::BinaryExpr(b) => {
                visit(Node::Expr(&b.left), visitor)?;
                if let Some(right) = &b.right {
                    visit(Node::Expr(right), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::UnaryExpr(u) => visit(Node::Expr(&u.expr), visitor),
            Node::ParenExpr(p) => visit(Node::Expr(&p.expr), visitor),
            Node::IsNullExpr(isn) => visit(Node::Expr(&isn.expr), visitor),
            Node::BetweenExpr(b) => {
                visit(Node::Expr(&b.expr), visitor)?;
                visit(Node::Expr(&b.low), visitor)?;
                visit(Node::Expr(&b.high), visitor)
            }
            Node::LikeExpr(l) => {
                visit(Node::Expr(&l.expr), visitor)?;
                visit(Node::Expr(&l.pattern), visitor)
            }
            Node::ILikeExpr(il) => {
                visit(Node::Expr(&il.expr), visitor)?;
                visit(Node::Expr(&il.pattern), visitor)
            }
            Node::SimilarExpr(s) => {
                visit(Node::Expr(&s.expr), visitor)?;
                visit(Node::Expr(&s.pattern), visitor)?;
                if let Some(escape) = &s.escape {
                    visit(Node::Expr(escape), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::FunctionCallExpr(fc) => {
                visit(Node::QualifiedName(&fc.name), visitor)?;
                for arg in &fc.args.items {
                    visit(Node::Expr(arg), visitor)?;
                }
                if let Some(filter) = &fc.filter {
                    visit(Node::Expr(filter), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::QuantifiedExpr(q) => visit(Node::Expr(&q.expr), visitor),
            Node::CaseExpr(c) => {
                if let Some(operand) = &c.operand {
                    visit(Node::Expr(operand), visitor)?;
                }
                for wc in &c.when_clauses {
                    visit(Node::Expr(&wc.when), visitor)?;
                    visit(Node::Expr(&wc.then), visitor)?;
                }
                if let Some(else_clause) = &c.else_clause {
                    visit(Node::Expr(else_clause), visitor)?;
                }
                ControlFlow::Continue(())
            }
            Node::InExpr(i) => {
                visit(Node::Expr(&i.expr), visitor)?;
                match &i.list {
                    ast::ExprList::Subquery(q) => visit(Node::Query(q), visitor),
                    ast::ExprList::Exprs(es) => {
                        for e in es {
                            visit(Node::Expr(e), visitor)?;
                        }
                        ControlFlow::Continue(())
                    }
                }
            }
            Node::OverExpr(o) => {
                visit(Node::QualifiedName(&o.name), visitor)?;
                for arg in &o.args.items {
                    visit(Node::Expr(arg), visitor)?;
                }
                match &o.over {
                    ast::WindowRef::Spec(spec) => visit(Node::WindowSpec(spec), visitor)?,
                    ast::WindowRef::Name(n) => visit(Node::SpannedStr(n), visitor)?,
                }
                if let Some(filter) = &o.filter {
                    visit(Node::Expr(filter), visitor)?;
                }
                ControlFlow::Continue(())
            }

            // Literals
            Node::Literal(_) => ControlFlow::Continue(()),

            // Names and identifiers
            Node::QualifiedName(_) => ControlFlow::Continue(()),
            Node::SpannedStr(_) => ControlFlow::Continue(()),
        }
    }
}

/// AST search operations.
impl<'a> Node<'a> {
    /// Finds the first node matching the predicate in pre-order (top-down)
    /// traversal.
    pub fn find<F>(&self, pred: F) -> Option<Node<'a>>
    where F: Fn(Node<'a>) -> bool {
        self.visit(&mut |event| match event {
            AstEvent::Enter(n) if pred(n) => ControlFlow::Break(n),
            _ => ControlFlow::Continue(()),
        })
        .break_value()
    }

    /// Finds the last node matching the predicate in post-order (bottom-up)
    /// traversal.
    pub fn find_rev<F>(&self, pred: F) -> Option<Node<'a>>
    where F: Fn(Node<'a>) -> bool {
        self.visit(&mut |event| match event {
            AstEvent::Exit(n) if pred(n) => ControlFlow::Break(n),
            _ => ControlFlow::Continue(()),
        })
        .break_value()
    }
}

/// AST traversal helpers.
impl<'a> Node<'a> {
    /// Filters and maps nodes during traversal, returning a vector of results.
    ///
    /// This is similar to Iterator's `filter_map`, but operates on the entire
    /// AST tree.
    pub fn filter_map<T, F>(&self, mut f: F) -> Vec<T>
    where F: FnMut(Node<'a>) -> Option<T> {
        let mut results = Vec::new();
        let _ = self.visit(&mut |event| {
            if let AstEvent::Enter(n) = event
                && let Some(val) = f(n)
            {
                results.push(val);
            }
            ControlFlow::Continue::<()>(())
        });
        results
    }

    /// Visits all nodes
    pub fn for_each<F>(&self, mut f: F)
    where F: FnMut(Node<'a>) {
        let _ = self.visit(&mut |event| {
            if let AstEvent::Enter(n) = event {
                f(n);
            }
            ControlFlow::Continue::<()>(())
        });
    }

    /// Visits only up to a given depth.
    pub fn for_each_max_depth<F>(&self, max_depth: usize, mut f: F)
    where F: FnMut(Node<'a>) {
        let mut depth = 0;
        let _ = self.visit(&mut |event| {
            match event {
                AstEvent::Enter(n) => {
                    if depth <= max_depth {
                        f(n);
                    }
                    depth += 1;
                }
                AstEvent::Exit(_) => {
                    depth -= 1;
                }
            }
            ControlFlow::Continue::<()>(())
        });
    }
}

/// AST transformation operations.
impl<'a> Node<'a> {
    /// Extracts the inner Select node from a Query, if present.
    pub fn as_select(&self) -> Option<&'a Loc<ast::Select>> {
        match self {
            Node::Query(q) => q.body.as_ref().and_then(|body| match &body.left.item {
                ast::QueryPrimary::Select(sel) => Some(sel),
                _ => None,
            }),
            _ => None,
        }
    }
}
