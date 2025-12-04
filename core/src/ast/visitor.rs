use std::ops::ControlFlow;

use crate::ast;
use crate::span::Loc;
use crate::span::Span;

/// Type-safe extraction and search for specific AST node types.
///
/// Automatically implemented for all AST types via `ast_node_impls!`.
pub trait AstNode<'a>: Sized + 'a {
    /// Extracts this node type from a `Node`, or `None` if it doesn't match.
    fn try_from_node(node: Node<'a>) -> Option<&'a Loc<Self>>;

    /// Checks if a `Node` is this type.
    fn is(node: Node<'a>) -> bool {
        Self::try_from_node(node).is_some()
    }

    /// Alias for `try_from_node`.
    fn cast(node: Node<'a>) -> Option<&'a Loc<Self>> {
        Self::try_from_node(node)
    }

    /// Finds the first node of this type (pre-order).
    fn find(node: Node<'a>) -> Option<&'a Loc<Self>> {
        node.find_map(Self::try_from_node)
    }

    /// Finds the first node of this type (pre-order) where the predicate is true.
    fn find_where(node: Node<'a>, pred: impl Fn(Node<'a>) -> bool) -> Option<&'a Loc<Self>> {
        node.find_map(|n| match pred(n) {
            true => Self::try_from_node(n),
            false => None,
        })
    }

    /// Finds the last node of this type (post-order).
    fn find_rev(node: Node<'a>) -> Option<&'a Loc<Self>> {
        node.find_map_rev(Self::try_from_node)
    }

    /// Finds the last node of this type (post-order) where the predicate is true.
    fn find_where_rev(
        node: impl Into<Node<'a>>, pred: impl Fn(&Loc<Self>) -> bool,
    ) -> Option<&'a Loc<Self>> {
        node.into().find_map_rev(|n| match Self::try_from_node(n) {
            Some(v) if pred(v) => Some(v),
            _ => None,
        })
    }

    /// Collects all nodes of this type (pre-order).
    fn find_all(node: Node<'a>) -> Vec<&'a Loc<Self>> {
        node.filter_map(Self::try_from_node)
    }

    /// Collects all nodes of this type (post-order).
    fn find_all_rev(node: Node<'a>) -> Vec<&'a Loc<Self>> {
        node.filter_map_rev(Self::try_from_node)
    }

    /// Collects all nodes of this type within the same query scope (excludes subqueries).
    fn find_all_same_query(node: Node<'a>) -> Vec<&'a Loc<Self>> {
        let mut out = Vec::new();
        node.find_same_query(|n| {
            if let Some(v) = Self::try_from_node(n) {
                out.push(v);
            }
            false
        });
        out
    }
}

impl<'a> Node<'a> {
    /// Casts this node to a specific AST type.
    pub fn cast<T: AstNode<'a>>(self) -> Option<&'a Loc<T>> {
        T::try_from_node(self)
    }
}

macro_rules! ast_node_impls {
    ($($n:ident),+ $(,)?) => {

        /// A reference to any node in the AST.
        ///
        /// This enum provides a unified type for traversing the entire SQL AST
        /// hierarchy. Each variant holds a borrowed reference to a specific node type,
        /// allowing zero-copy tree traversal.
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum Node<'a> {
            $( $n(&'a Loc<ast::$n>), )*
        }

        $(
            impl<'a> AstNode<'a> for ast::$n {
                fn try_from_node(node: Node<'a>) -> Option<&'a Loc<Self>> {
                    match node { Node::$n(x) => Some(x), _ => None }
                }
            }

            impl<'a> From<&'a Loc<ast::$n>> for Node<'a> {
                fn from(t: &'a Loc<ast::$n>) -> Self { Node::$n(t) }
            }
        )+


        impl<'a> Node<'a> {
            /// Returns the source span of this node.
            pub fn span(&self) -> Span {
                match self {
                    $(Node::$n(n) => n.span,)+
                }
            }

            /// Returns the name of this node variant.
            pub fn name(&self) -> &'static str {
                match self {
                    $(Node::$n(_) => stringify!($n),)+
                }
            }
        }
    };
}

#[rustfmt::skip]
ast_node_impls!(
    Statement, Query, With, Cte, SetOpTerm, QueryPrimary,
    Select, Projection, ProjectionItem,
    Insert, Update, Delete, Assignment,
    From, TableRef, TableFactor, NamedTableFactor, FunctionTableFactor, SubqueryTableFactor, Join, JoinConstraint,
    Where,
    GroupBy, GroupByItem, GroupingSet,
    Window, WindowDef, WindowSpec, WindowFrame, FrameBound,
    QuerySuffix, OrderBy, OrderByItem, Limit, Offset,
    Values,
    Expr, Binary, Unary, Paren, IsNull, Between, Like, ILike, Similar,
    FunctionCall, Quantified, Case, In, Over, Cast, Subscript, Row, AtTimeZone,
    Literal, QualifiedName, Identifier
);

/// Events emitted during AST traversal.
#[derive(Debug, Clone, Copy)]
pub enum VisitEvent<'a> {
    /// Entering a node (before visiting children).
    Enter(Node<'a>),
    /// Exiting a node (after visiting children).
    Exit(Node<'a>),
}

/// Control flow indicator for selective AST traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WalkCtl {
    /// Continue traversing into child nodes.
    Continue,
    /// Skip visiting the children of the current node.
    Skip,
}

type Flow<T> = ControlFlow<T, WalkCtl>;

impl WalkCtl {
    #[inline]
    pub fn cont<T>() -> Flow<T> {
        Flow::<T>::Continue(WalkCtl::Continue)
    }
    #[inline]
    pub fn skip<T>() -> Flow<T> {
        Flow::<T>::Continue(WalkCtl::Skip)
    }
}

/// AST traversal operations.
impl<'a> Node<'a> {
    /// Depth-first traversal with subtree skipping support.
    fn walk_selective<T>(&self, vis: &mut impl FnMut(VisitEvent<'a>) -> Flow<T>) -> ControlFlow<T> {
        self.for_each_child(|child| {
            match vis(VisitEvent::Enter(child))? {
                WalkCtl::Continue => {
                    child.walk_selective(vis)?;
                }
                WalkCtl::Skip => {}
            }
            let _ = vis(VisitEvent::Exit(child))?;
            ControlFlow::Continue(())
        })
    }

    /// Reverse depth-first traversal with subtree skipping support.
    #[allow(dead_code)]
    fn walk_selective_rev<T>(
        &self, vis: &mut impl FnMut(VisitEvent<'a>) -> Flow<T>,
    ) -> ControlFlow<T> {
        self.for_each_child(|child| {
            let _ = vis(VisitEvent::Enter(child))?;
            match vis(VisitEvent::Exit(child))? {
                WalkCtl::Continue => {
                    child.walk_selective(vis)?;
                }
                WalkCtl::Skip => {}
            }
            ControlFlow::Continue(())
        })
    }

    /// Standard depth-first traversal.
    fn walk<T>(&self, vis: &mut impl FnMut(VisitEvent<'a>) -> ControlFlow<T>) -> ControlFlow<T> {
        self.for_each_child(|child| {
            vis(VisitEvent::Enter(child))?;
            child.walk(vis)?;
            vis(VisitEvent::Exit(child))
        })
    }

    /// Finds the first node matching the predicate (pre-order).
    pub fn find(&self, pred: impl Fn(Node<'a>) -> bool) -> Option<Node<'a>> {
        self.walk(&mut |ev| match ev {
            VisitEvent::Enter(n) if pred(n) => ControlFlow::Break(n),
            _ => ControlFlow::Continue(()),
        })
        .break_value()
    }

    /// Finds the last node matching the predicate (post-order).
    pub fn find_rev(&self, pred: impl Fn(Node<'a>) -> bool) -> Option<Node<'a>> {
        self.walk(&mut |ev| match ev {
            VisitEvent::Exit(n) if pred(n) => ControlFlow::Break(n),
            _ => ControlFlow::Continue(()),
        })
        .break_value()
    }

    /// Finds nodes within the same query scope (excludes subqueries).
    pub fn find_same_query(&self, mut pred: impl FnMut(Node<'a>) -> bool) -> Option<Node<'a>> {
        self.walk_selective(&mut |ev| match ev {
            VisitEvent::Enter(n) if matches!(n, Node::Query(_)) => WalkCtl::skip::<Node<'_>>(),
            VisitEvent::Enter(n) if pred(n) => ControlFlow::Break(n),
            _ => WalkCtl::cont::<Node<'_>>(),
        })
        .break_value()
    }

    /// Finds and transforms the first matching node (pre-order).
    pub fn find_map<T>(&self, pred: impl Fn(Node<'a>) -> Option<T>) -> Option<T> {
        self.walk(&mut |ev| match ev {
            VisitEvent::Enter(n) => pred(n).map_or(ControlFlow::Continue(()), ControlFlow::Break),
            _ => ControlFlow::Continue(()),
        })
        .break_value()
    }

    /// Finds and transforms the last matching node (post-order).
    pub fn find_map_rev<T>(&self, pred: impl Fn(Node<'a>) -> Option<T>) -> Option<T> {
        self.walk(&mut |ev| match ev {
            VisitEvent::Exit(n) => pred(n).map_or(ControlFlow::Continue(()), ControlFlow::Break),
            _ => ControlFlow::Continue(()),
        })
        .break_value()
    }

    /// Filters and maps nodes (pre-order).
    pub fn filter_map<T>(&self, mut f: impl FnMut(Node<'a>) -> Option<T>) -> Vec<T> {
        let mut out = Vec::new();
        let _: ControlFlow<()> = self.walk(&mut |ev| {
            if let VisitEvent::Enter(n) = ev {
                if let Some(v) = f(n) {
                    out.push(v);
                }
            }
            ControlFlow::Continue(())
        });
        out
    }

    /// Filters and maps nodes (post-order).
    pub fn filter_map_rev<T>(&self, mut f: impl FnMut(Node<'a>) -> Option<T>) -> Vec<T> {
        let mut out = Vec::new();
        let _: ControlFlow<()> = self.walk(&mut |ev| {
            if let VisitEvent::Exit(n) = ev {
                if let Some(v) = f(n) {
                    out.push(v);
                }
            }
            ControlFlow::Continue(())
        });
        out
    }

    /// Folds over nodes, accumulating state (pre-order).
    pub fn fold<B>(&self, mut state: B, mut f: impl FnMut(&mut B, Node<'a>) -> Option<()>) -> B {
        let _ = self.walk(&mut |ev| match ev {
            VisitEvent::Enter(n) => {
                f(&mut state, n).map_or(ControlFlow::Break(()), |_| ControlFlow::Continue(()))
            }
            VisitEvent::Exit(_) => ControlFlow::Continue(()),
        });
        state
    }

    /// Folds over nodes, accumulating state (post-order).
    pub fn fold_rev<B>(
        &self, mut state: B, mut f: impl FnMut(&mut B, Node<'a>) -> Option<()>,
    ) -> B {
        let _ = self.walk(&mut |ev| match ev {
            VisitEvent::Exit(n) => {
                f(&mut state, n).map_or(ControlFlow::Break(()), |_| ControlFlow::Continue(()))
            }
            VisitEvent::Enter(_) => ControlFlow::Continue(()),
        });
        state
    }

    /// Returns a pre-order iterator over all nodes.
    pub fn preorder(&'a self) -> impl Iterator<Item = Node<'a>> + 'a {
        let mut stack = vec![*self];
        std::iter::from_fn(move || {
            if let Some(top) = stack.pop() {
                let mut first_child: Option<Node<'a>> = None;
                let mut rest: Vec<Node<'a>> = Vec::new();
                let _ = top.walk_selective(&mut |ev| match ev {
                    VisitEvent::Enter(n) => {
                        if first_child.is_none() {
                            first_child = Some(n);
                        } else {
                            rest.push(n);
                        }
                        WalkCtl::skip::<()>()
                    }
                    VisitEvent::Exit(_) => WalkCtl::cont::<()>(),
                });
                // Push children in reverse so the first child is visited next.
                for c in rest.into_iter().rev() {
                    stack.push(c);
                }
                if let Some(c) = first_child {
                    stack.push(c);
                }
                Some(top)
            } else {
                None
            }
        })
    }

    /// Returns a post-order iterator over all nodes.
    pub fn postorder(&'a self) -> impl Iterator<Item = Node<'a>> + 'a {
        let mut out = Vec::new();
        let _: ControlFlow<()> = self.walk(&mut |ev| {
            if let VisitEvent::Exit(n) = ev {
                out.push(n);
            }
            ControlFlow::Continue(())
        });
        out.into_iter()
    }

    /// Calls `f` for each immediate child node.
    fn for_each_child<T>(&self, mut f: impl FnMut(Node<'a>) -> ControlFlow<T>) -> ControlFlow<T> {
        match self {
            // Top-level
            Node::Statement(stmt) => match &stmt.item {
                ast::Statement::Query(n) => f(Node::Query(n)),
                ast::Statement::Insert(n) => f(Node::Insert(n)),
                ast::Statement::Update(n) => f(Node::Update(n)),
                ast::Statement::Delete(n) => f(Node::Delete(n)),
                ast::Statement::Partial(_) => ControlFlow::Continue(()),
            },

            // Query structure
            Node::Query(n) => {
                if let Some(with) = &n.item.with {
                    f(Node::With(with))?;
                }
                if let Some(body) = &n.item.body {
                    f(Node::QueryPrimary(&body.left))?;
                    for chain in &body.set_ops {
                        f(Node::SetOpTerm(chain))?;
                    }
                }
                if let Some(tail) = &n.item.tail {
                    f(Node::QuerySuffix(tail))?;
                }
                ControlFlow::Continue(())
            }
            Node::With(n) => {
                for cte in &n.ctes {
                    f(Node::Cte(cte))?;
                }
                ControlFlow::Continue(())
            }
            Node::Cte(n) => {
                if let Some(columns) = &n.columns {
                    for column in &columns.items {
                        f(Node::Identifier(column))?;
                    }
                }
                f(Node::Query(&n.query))
            }
            Node::SetOpTerm(n) => f(Node::QueryPrimary(&n.right)),
            Node::QueryPrimary(query) => match &query.item {
                ast::QueryPrimary::Select(n) => f(Node::Select(n)),
                ast::QueryPrimary::Values(n) => f(Node::Values(n)),
                ast::QueryPrimary::Parenthesized(n) => f(Node::Query(n)),
            },

            // SELECT statement
            Node::Select(stmt) => {
                // Visit DISTINCT ON expressions if present
                if let ast::SetQuantifier::DistinctOn(ref exprs) = stmt.distinct {
                    for expr in &exprs.items {
                        f(Node::Expr(expr))?;
                    }
                }
                f(Node::Projection(&stmt.projection))?;
                if let Some(from) = &stmt.from {
                    f(Node::From(from))?;
                }
                if let Some(where_clause) = &stmt.where_clause {
                    f(Node::Where(where_clause))?;
                }
                if let Some(group_by) = &stmt.group_by {
                    f(Node::GroupBy(group_by))?;
                }
                if let Some(having) = &stmt.having {
                    f(Node::Expr(having))?;
                }
                if let Some(window) = &stmt.window {
                    f(Node::Window(window))?;
                }
                if let Some(qualify) = &stmt.qualify {
                    f(Node::Expr(qualify))?;
                }
                ControlFlow::Continue(())
            }
            Node::Projection(n) => {
                for item in &n.list.items {
                    f(Node::ProjectionItem(item))?;
                }
                ControlFlow::Continue(())
            }
            Node::ProjectionItem(n) => f(Node::Expr(&n.expr)),

            // DML statements
            Node::Insert(n) => {
                f(Node::QualifiedName(&n.table))?;
                match &n.source {
                    ast::InsertSource::Values(v) => f(Node::Values(v))?,
                    ast::InsertSource::Query(q) => f(Node::Query(q))?,
                    ast::InsertSource::Default => {}
                }
                if let Some(ret) = &n.returning {
                    f(Node::Projection(ret))?;
                }
                ControlFlow::Continue(())
            }
            Node::Update(n) => {
                f(Node::QualifiedName(&n.table))?;
                for assignment in &n.assignments.items {
                    f(Node::Assignment(assignment))?;
                }
                if let Some(from) = &n.from {
                    f(Node::From(from))?;
                }
                if let Some(where_clause) = &n.where_clause {
                    f(Node::Where(where_clause))?;
                }
                if let Some(ret) = &n.returning {
                    f(Node::Projection(ret))?;
                }
                ControlFlow::Continue(())
            }
            Node::Delete(n) => {
                f(Node::QualifiedName(&n.table))?;
                if let Some(using) = &n.using {
                    f(Node::From(using))?;
                }
                if let Some(where_clause) = &n.where_clause {
                    f(Node::Where(where_clause))?;
                }
                if let Some(ret) = &n.returning {
                    f(Node::Projection(ret))?;
                }
                ControlFlow::Continue(())
            }
            Node::Assignment(n) => f(Node::Expr(&n.value)),

            // FROM clause
            Node::From(n) => {
                for source in &n.sources.items {
                    f(Node::TableRef(source))?;
                }
                ControlFlow::Continue(())
            }
            Node::TableRef(n) => match &n.item {
                ast::TableRef::Factor(t) => f(Node::TableFactor(t)),
                ast::TableRef::Join(j) => f(Node::Join(j)),
            },
            Node::TableFactor(p) => match &p.item {
                ast::TableFactor::Named(n) => f(Node::NamedTableFactor(n)),
                ast::TableFactor::Function(n) => f(Node::FunctionTableFactor(n)),
                ast::TableFactor::Subquery(n) => f(Node::SubqueryTableFactor(n)),
                ast::TableFactor::Parenthesized(n) => f(Node::TableRef(n)),
            },
            Node::NamedTableFactor(n) => f(Node::QualifiedName(&n.name)),
            Node::FunctionTableFactor(n) => {
                f(Node::QualifiedName(&n.name))?;
                for arg in &n.args.items {
                    f(Node::Expr(arg))?;
                }
                if let Some(columns) = &n.columns {
                    for column in &columns.items {
                        f(Node::Identifier(column))?;
                    }
                }
                ControlFlow::Continue(())
            }
            Node::SubqueryTableFactor(n) => f(Node::Query(&n.query)),
            Node::Join(n) => {
                f(Node::TableRef(&n.left))?;
                f(Node::TableRef(&n.right))?;
                if let Some(constraint) = &n.constraint {
                    f(Node::JoinConstraint(constraint))?;
                }
                ControlFlow::Continue(())
            }
            Node::JoinConstraint(n) => match &n.item {
                ast::JoinConstraint::On(e) => f(Node::Expr(e)),
                ast::JoinConstraint::Using(es) => {
                    for e in &es.items {
                        f(Node::Identifier(e))?;
                    }
                    ControlFlow::Continue(())
                }
            },

            // WHERE clause
            Node::Where(n) => f(Node::Expr(&n.expr)),

            // GROUP BY clause
            Node::GroupBy(n) => {
                for item in &n.items.items {
                    f(Node::GroupByItem(item))?;
                }
                ControlFlow::Continue(())
            }
            Node::GroupByItem(n) => match &n.item {
                ast::GroupByItem::Expr(e) => f(Node::Expr(e)),
                ast::GroupByItem::Rollup(es) | ast::GroupByItem::Cube(es) => {
                    for e in es {
                        f(Node::Expr(e))?;
                    }
                    ControlFlow::Continue(())
                }
                ast::GroupByItem::GroupingSets(sets) => {
                    for gs in sets {
                        f(Node::GroupingSet(gs))?;
                    }
                    ControlFlow::Continue(())
                }
            },
            Node::GroupingSet(n) => match &n.item {
                ast::GroupingSet::Expr(e) => f(Node::Expr(e)),
                ast::GroupingSet::Exprs(es) => {
                    for e in es {
                        f(Node::Expr(e))?;
                    }
                    ControlFlow::Continue(())
                }
            },

            // WINDOW clause
            Node::Window(n) => {
                for window in &n.windows {
                    f(Node::WindowDef(window))?;
                }
                ControlFlow::Continue(())
            }
            Node::WindowDef(n) => f(Node::WindowSpec(&n.spec)),
            Node::WindowSpec(n) => {
                if let Some(pb) = &n.partition_by {
                    for item in &pb.items {
                        f(Node::Expr(item))?;
                    }
                }
                if let Some(ob) = &n.order_by {
                    f(Node::OrderBy(ob))?;
                }
                if let Some(frame) = &n.frame {
                    f(Node::WindowFrame(frame))?;
                }
                ControlFlow::Continue(())
            }
            Node::WindowFrame(n) => {
                f(Node::FrameBound(&n.start))?;
                if let Some(end) = &n.end {
                    f(Node::FrameBound(end))?;
                }
                ControlFlow::Continue(())
            }
            Node::FrameBound(n) => match &n.item {
                ast::FrameBound::Preceding(e) | ast::FrameBound::Following(e) => f(Node::Expr(e)),
                _ => ControlFlow::Continue(()),
            },

            // ORDER BY / LIMIT / OFFSET
            Node::QuerySuffix(n) => {
                if let Some(order_by) = &n.order_by {
                    f(Node::OrderBy(order_by))?;
                }
                if let Some(limit) = &n.limit {
                    f(Node::Limit(limit))?;
                }
                if let Some(offset) = &n.offset {
                    f(Node::Offset(offset))?;
                }
                ControlFlow::Continue(())
            }
            Node::OrderBy(n) => {
                for item in &n.items.items {
                    f(Node::OrderByItem(item))?;
                }
                ControlFlow::Continue(())
            }
            Node::OrderByItem(n) => f(Node::Expr(&n.expr)),
            Node::Limit(n) => f(Node::Expr(&n.count)),
            Node::Offset(n) => f(Node::Expr(&n.count)),

            // VALUES statement
            Node::Values(stmt) => {
                for row in &stmt.rows {
                    for item in &row.items {
                        f(Node::Expr(item))?;
                    }
                }
                ControlFlow::Continue(())
            }

            // Expressions
            Node::Expr(e) => match &e.item {
                ast::Expr::Name(n) => f(Node::QualifiedName(n)),
                ast::Expr::Literal(l) => f(Node::Literal(l)),
                ast::Expr::Binary(b) => f(Node::Binary(b)),
                ast::Expr::Unary(u) => f(Node::Unary(u)),
                ast::Expr::Paren(p) => f(Node::Paren(p)),
                ast::Expr::Subquery(q) => f(Node::Query(q)),
                ast::Expr::IsNull(isn) => f(Node::IsNull(isn)),
                ast::Expr::Between(b) => f(Node::Between(b)),
                ast::Expr::Like(l) => f(Node::Like(l)),
                ast::Expr::ILike(il) => f(Node::ILike(il)),
                ast::Expr::Similar(s) => f(Node::Similar(s)),
                ast::Expr::FunctionCall(fc) => f(Node::FunctionCall(fc)),
                ast::Expr::Array(items) => {
                    for item in &items.items {
                        f(Node::Expr(item))?;
                    }
                    ControlFlow::Continue(())
                }
                ast::Expr::Quantified(q) => f(Node::Quantified(q)),
                ast::Expr::Case(c) => f(Node::Case(c)),
                ast::Expr::In(i) => f(Node::In(i)),
                ast::Expr::Over(o) => f(Node::Over(o)),
                ast::Expr::Exists(q) => f(Node::Query(q)),
                ast::Expr::Cast(c) => f(Node::Cast(c)),
                ast::Expr::Subscript(s) => f(Node::Subscript(s)),
                ast::Expr::Row(r) => f(Node::Row(r)),
                ast::Expr::AtTimeZone(atz) => f(Node::AtTimeZone(atz)),
                ast::Expr::Empty => ControlFlow::Continue(()),
            },
            Node::Cast(c) => f(Node::Expr(&c.expr)),
            Node::Subscript(s) => {
                f(Node::Expr(&s.expr))?;
                f(Node::Expr(&s.index))?;
                if let Some(upper) = &s.upper {
                    f(Node::Expr(upper))?;
                }
                ControlFlow::Continue(())
            }
            Node::Row(r) => {
                for e in &r.exprs.items {
                    f(Node::Expr(e))?;
                }
                ControlFlow::Continue(())
            }
            Node::AtTimeZone(atz) => {
                f(Node::Expr(&atz.expr))?;
                f(Node::Expr(&atz.timezone))
            }
            Node::Binary(b) => {
                f(Node::Expr(&b.left))?;
                if let Some(right) = &b.right {
                    f(Node::Expr(right))?;
                }
                ControlFlow::Continue(())
            }
            Node::Unary(u) => f(Node::Expr(&u.expr)),
            Node::Paren(p) => f(Node::Expr(&p.expr)),
            Node::IsNull(isn) => f(Node::Expr(&isn.expr)),
            Node::Between(b) => {
                f(Node::Expr(&b.expr))?;
                f(Node::Expr(&b.low))?;
                f(Node::Expr(&b.high))
            }
            Node::Like(l) => {
                f(Node::Expr(&l.expr))?;
                f(Node::Expr(&l.pattern))
            }
            Node::ILike(il) => {
                f(Node::Expr(&il.expr))?;
                f(Node::Expr(&il.pattern))
            }
            Node::Similar(s) => {
                f(Node::Expr(&s.expr))?;
                f(Node::Expr(&s.pattern))?;
                if let Some(escape) = &s.escape {
                    f(Node::Expr(escape))?;
                }
                ControlFlow::Continue(())
            }
            Node::FunctionCall(fc) => {
                f(Node::QualifiedName(&fc.name))?;
                for arg in &fc.args.items {
                    f(Node::Expr(arg))?;
                }
                if let Some(filter) = &fc.filter {
                    f(Node::Expr(filter))?;
                }
                ControlFlow::Continue(())
            }
            Node::Quantified(q) => f(Node::Expr(&q.expr)),
            Node::Case(c) => {
                if let Some(operand) = &c.operand {
                    f(Node::Expr(operand))?;
                }
                for wc in &c.when_clauses {
                    f(Node::Expr(&wc.when))?;
                    f(Node::Expr(&wc.then))?;
                }
                if let Some(else_clause) = &c.else_clause {
                    f(Node::Expr(else_clause))?;
                }
                ControlFlow::Continue(())
            }
            Node::In(i) => {
                f(Node::Expr(&i.expr))?;
                match &i.list {
                    ast::ExprList::Subquery(q) => f(Node::Query(q)),
                    ast::ExprList::Exprs(es) => {
                        for e in es {
                            f(Node::Expr(e))?;
                        }
                        ControlFlow::Continue(())
                    }
                }
            }
            Node::Over(o) => {
                f(Node::QualifiedName(&o.name))?;
                for arg in &o.args.items {
                    f(Node::Expr(arg))?;
                }
                match &o.over {
                    ast::WindowRef::Spec(spec) => f(Node::WindowSpec(spec))?,
                    ast::WindowRef::Name(n) => f(Node::Identifier(n))?,
                }
                if let Some(filter) = &o.filter {
                    f(Node::Expr(filter))?;
                }
                ControlFlow::Continue(())
            }

            // Literals
            Node::Literal(_) => ControlFlow::Continue(()),

            // Names and identifiers
            Node::QualifiedName(_) => ControlFlow::Continue(()),
            Node::Identifier(_) => ControlFlow::Continue(()),
        }
    }
}

/// AST transformation operations.
impl<'a> Node<'a> {
    /// Extracts the SELECT statement from a Query node, if present.
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
