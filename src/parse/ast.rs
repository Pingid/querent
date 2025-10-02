use std::ops::Deref;

use crate::tokenize::{OpTag, Span, TokenKind};

/// Identifier is a zero-copy string slice tracked by Span
pub type Ident = Span;

/// SQL Statement types
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Query(Node<Query>),
    Partial(Node<()>),
}

/// A full query: optional WITH, a set-operation expression body, and optional tail (ORDER/LIMIT/OFFSET)
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub with: Option<Node<With>>,
    pub body: Option<Node<QueryExpr>>,
    pub tail: Option<Node<QueryTail>>,
}

/// Query expression as a chain of set operations (flattened)
#[derive(Debug, Clone, PartialEq)]
pub struct QueryExpr {
    pub left: Node<QueryCore>,
    pub set_ops: Vec<SetOpChain>,
}

/// A single set operation in the chain
#[derive(Debug, Clone, PartialEq)]
pub struct SetOpChain {
    pub op: SetOp,
    pub right: Node<QueryCore>,
}

/// Core query without set operations
#[derive(Debug, Clone, PartialEq)]
pub enum QueryCore {
    Select(SelectStmt),
    Values(ValuesStmt),
    Parenthesized(Box<Query>),
}

/// SELECT statement
#[derive(Debug, Clone, PartialEq)]
pub struct SelectStmt {
    pub distinct: Distinct,
    pub projection: Node<DelimitedList<Node<SelectItem>>>,
    pub from: Option<Node<FromClause>>,
    pub where_clause: Option<Node<Expr>>,
    pub group_by: Option<Node<GroupByClause>>,
    pub having: Option<Node<Expr>>,
    pub window: Option<Node<WindowClause>>,
    pub qualify: Option<Node<Expr>>,
}

/// WITH clause containing CTEs
#[derive(Debug, Clone, PartialEq)]
pub struct With {
    pub recursive: bool,
    pub ctes: Vec<Node<CTE>>,
}

/// Common Table Expression (CTE)
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub struct CTE {
    pub name: Ident,
    pub columns: Option<DelimitedList<Node<Ident>>>,
    pub materialized: Option<Materialized>,
    pub query: Box<Node<Query>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Materialized {
    Materialized,
    NotMaterialized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    Union { all: bool },
    Intersect { all: bool },
    Except { all: bool },
    Minus { all: bool },
}

/// VALUES statement
#[derive(Debug, Clone, PartialEq)]
pub struct ValuesStmt {
    pub rows: Vec<Node<DelimitedList<Node<Expr>>>>,
}

/// Query tail with ORDER BY, LIMIT, OFFSET (binds to the whole query expression)
#[derive(Debug, Clone, PartialEq)]
pub struct QueryTail {
    pub order_by: Option<Node<OrderByClause>>,
    pub limit: Option<Node<LimitClause>>,
    pub offset: Option<Node<OffsetClause>>,
}

/// ------------- ORDER / LIMIT / OFFSET -------------

#[derive(Debug, Clone, PartialEq)]
pub struct OrderByClause {
    pub items: DelimitedList<Node<OrderByItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderByItem {
    pub expr: Node<Expr>,
    pub direction: Option<OrderDirection>,
    pub nulls: Option<NullOrdering>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullOrdering {
    First,
    Last,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LimitClause {
    pub count: Node<Expr>,
    pub style: LimitStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OffsetClause {
    pub count: Node<Expr>,
    pub rows_keyword: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitStyle {
    FetchFirst,
    Limit,
}

/// ------------- SELECT parts -------------

#[derive(Debug, Clone, PartialEq)]
pub enum Distinct {
    All,
    Distinct,
    DistinctOn(DelimitedList<Node<Expr>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectItem {
    pub expr: Node<Expr>,
    pub alias: Option<Node<Ident>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FromClause {
    pub sources: DelimitedList<Node<TableRef>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupByClause {
    pub items: DelimitedList<Node<GroupByItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowClause {
    pub windows: Vec<Node<WindowDef>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowDef {
    pub name: Ident,
    pub spec: Node<WindowSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowSpec {
    pub partition_by: Option<DelimitedList<Node<Expr>>>,
    pub order_by: Option<Node<OrderByClause>>,
    pub frame: Option<Box<Node<WindowFrame>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowFrame {
    pub unit: FrameUnit,
    pub start: Box<Node<FrameBound>>,
    pub end: Option<Box<Node<FrameBound>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameUnit {
    Rows,
    Range,
    Groups,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FrameBound {
    UnboundedPreceding,
    Preceding(Box<Node<Expr>>),
    CurrentRow,
    Following(Box<Node<Expr>>),
    UnboundedFollowing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GroupByItem {
    Expr(Node<Expr>),
    Rollup(Vec<Node<Expr>>),
    Cube(Vec<Node<Expr>>),
    GroupingSets(Vec<Node<GroupingSet>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GroupingSet {
    Expr(Node<Expr>),
    Exprs(Vec<Node<Expr>>),
}

/// ------------- Table references & joins -------------

/// A table reference is a join tree over table factors
#[derive(Debug, Clone, PartialEq)]
pub enum TableRef {
    /// Leaf node (named table, subquery, or parenthesized group)
    Factor(Node<TableFactor>),
    /// Binary join
    Join {
        left: Box<Node<TableRef>>,
        kind: JoinKind,
        right: Box<Node<TableRef>>,
        constraint: Option<Node<JoinConstraint>>,
    },
}

/// Table factors that can appear as leaves in the join tree
#[derive(Debug, Clone, PartialEq)]
pub enum TableFactor {
    Named {
        name: Node<QualifiedName>,
        alias: Option<Node<Ident>>,
        lateral: bool,
    },
    Function {
        name: Node<QualifiedName>,
        args: DelimitedList<Node<Expr>>,
        alias: Option<Node<Ident>>,
        columns: Option<DelimitedList<Node<Ident>>>,
        lateral: bool,
    },
    Subquery {
        query: Node<Query>,
        alias: Option<Node<Ident>>,
        lateral: bool,
    },
    Parenthesized {
        inner: Box<Node<TableRef>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JoinKind {
    pub base: JoinBase,
    pub outer: bool,
    pub natural: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinBase {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JoinConstraint {
    On(Node<Expr>),
    Using(DelimitedList<Node<Ident>>),
}

/// ------------- Expressions & names -------------
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Column(Node<QualifiedName>),
    Literal(Literal),
    Binary {
        left: Box<Node<Expr>>,
        op: Option<OpTag>,
        right: Option<Box<Node<Expr>>>,
    },
    Unary {
        op_tok: Node<OpTag>,
        expr: Box<Node<Expr>>,
    },
    Paren {
        open: Ident,
        expr: Box<Node<Expr>>,
        close: Option<Ident>,
    },
    Subquery(Box<Node<Query>>),
    IsNull {
        expr: Box<Node<Expr>>,
        not: bool,
    },
    Between {
        expr: Box<Node<Expr>>,
        low: Box<Node<Expr>>,
        high: Box<Node<Expr>>,
        not: bool,
    },
    Like {
        expr: Box<Node<Expr>>,
        pattern: Box<Node<Expr>>,
        not: bool,
    },
    /// Postgres ILIKE (case-insensitive LIKE)
    ILike {
        expr: Box<Node<Expr>>,
        pattern: Box<Node<Expr>>,
        not: bool,
    },
    /// SIMILAR TO [ESCAPE]
    Similar {
        expr: Box<Node<Expr>>,
        pattern: Box<Node<Expr>>,
        escape: Option<Box<Node<Expr>>>,
    },
    FunctionCall {
        name: Node<QualifiedName>,
        distinct: bool,
        args: DelimitedList<Node<Expr>>,
        filter: Option<Box<Node<Expr>>>,
    },
    /// ARRAY[...] constructor
    Array(DelimitedList<Node<Expr>>),
    /// Quantified expressions: ANY(...), SOME(...), ALL(...)
    Quantified {
        quantifier: Quantifier,
        expr: Box<Node<Expr>>,
    },
    Case {
        operand: Option<Box<Node<Expr>>>,
        when_clauses: Vec<WhenClause>,
        else_clause: Option<Box<Node<Expr>>>,
    },
    In {
        expr: Box<Node<Expr>>,
        list: InList,
        not: bool,
    },
    WindowFunction {
        name: Node<QualifiedName>,
        args: DelimitedList<Node<Expr>>,
        over: WindowOver,
        filter: Option<Box<Node<Expr>>>,
    },
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindowOver {
    Spec(Box<Node<WindowSpec>>),
    Name(Node<Ident>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantifier {
    Any,
    Some,
    All,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InList {
    Subquery(Box<Node<Query>>),
    Exprs(Vec<Node<Expr>>),
}

/// WHEN ... THEN ... clause in a CASE expression
#[derive(Debug, Clone, PartialEq)]
pub struct WhenClause {
    pub when: Node<Expr>,
    pub then: Node<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(i64),
    Float(f64),
    String(Ident),
    Null,
    Boolean(Boolean),
    TypedString {
        data_type: TypedLiteralKind,
        value: Ident,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boolean {
    True,
    False,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedLiteralKind {
    Date,
    Time,
    Timestamp,
}

/// Name such as schema.table.column or simply column; supports STAR
#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedName {
    pub parts: DelimitedList<Node<NamePart>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamePart {
    Ident(Ident),
    Star,
}

/// ------------- Utility containers -------------
/// A node with span
#[derive(Debug, Clone, PartialEq)]
pub struct Node<T> {
    pub span: Ident,
    pub item: T,
}

/// A list of items separated by a separator
#[derive(Debug, Clone, PartialEq)]
pub struct DelimitedList<T> {
    pub items: Vec<T>,
    pub seps: Vec<Node<TokenKind>>,
}

// ---------------- Implementations ----------------
impl<T> Node<T> {
    pub fn new(span: impl Into<Span>, kind: T) -> Self {
        let span = span.into();
        Node { span, item: kind }
    }
}

pub fn node<T>(span: impl Into<Span>, kind: impl Into<T>) -> Node<T> {
    Node::new(span, kind.into())
}

impl<T> Deref for Node<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

// DelimitedList
impl<T> Default for DelimitedList<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            seps: Vec::new(),
        }
    }
}
