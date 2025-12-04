use crate::lex::OpTag;
use crate::lex::TokenKind;
use crate::span::Loc;
use crate::span::Span;

/// Identifier is a zero-copy string slice tracked by Span
pub type Identifier = Span;

/// Vector type for AST nodes (kept as Vec - SmallVec proved slower due to
/// struct size bloat hurting cache performance during parsing)
pub type AstVec<T> = Vec<T>;

// ------------- Top-level statement -------------

/// SQL Statement types
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Query(Loc<Query>),
    Insert(Loc<Insert>),
    Update(Loc<Update>),
    Delete(Loc<Delete>),
    Partial(Loc<()>),
}

// ------------- DML Statements -------------

/// INSERT statement
#[derive(Debug, Clone, PartialEq)]
pub struct Insert {
    pub table: Loc<QualifiedName>,
    pub columns: Option<DelimitedList<Loc<Identifier>>>,
    pub source: InsertSource,
    pub returning: Option<Loc<Projection>>,
}

/// Source of data for INSERT
#[derive(Debug, Clone, PartialEq)]
pub enum InsertSource {
    Values(Loc<Values>),
    Query(Box<Loc<Query>>),
    Default,
}

/// UPDATE statement
#[derive(Debug, Clone, PartialEq)]
pub struct Update {
    pub table: Loc<QualifiedName>,
    pub alias: Option<Loc<Identifier>>,
    pub assignments: DelimitedList<Loc<Assignment>>,
    pub from: Option<Loc<From>>,
    pub where_clause: Option<Loc<Where>>,
    pub returning: Option<Loc<Projection>>,
}

/// Column assignment in UPDATE SET clause
#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub column: Loc<Identifier>,
    pub value: Loc<Expr>,
}

/// DELETE statement
#[derive(Debug, Clone, PartialEq)]
pub struct Delete {
    pub table: Loc<QualifiedName>,
    pub alias: Option<Loc<Identifier>>,
    pub using: Option<Loc<From>>,
    pub where_clause: Option<Loc<Where>>,
    pub returning: Option<Loc<Projection>>,
}

// ------------- Query structure -------------

/// A full query: optional WITH, a set-operation expression body, and optional
/// tail (ORDER/LIMIT/OFFSET)
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub with: Option<Loc<With>>,
    pub body: Option<Loc<QueryExpr>>,
    pub tail: Option<Loc<QuerySuffix>>,
}

/// WITH clause containing CTEs
#[derive(Debug, Clone, PartialEq)]
pub struct With {
    pub recursive: bool,
    pub ctes: AstVec<Loc<Cte>>,
}

/// Common Table Expression (CTE)
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub struct Cte {
    pub name: Identifier,
    pub columns: Option<DelimitedList<Loc<Identifier>>>,
    pub materialized: Option<CteMaterialization>,
    pub query: Box<Loc<Query>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CteMaterialization {
    Materialized,
    NotMaterialized,
}

/// Query expression as a chain of set operations (flattened)
#[derive(Debug, Clone, PartialEq)]
pub struct QueryExpr {
    pub left: Loc<QueryPrimary>,
    pub set_ops: AstVec<Loc<SetOpTerm>>,
}

/// A single set operation in the chain
#[derive(Debug, Clone, PartialEq)]
pub struct SetOpTerm {
    pub op: SetOp,
    pub right: Loc<QueryPrimary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    Union { all: bool },
    Intersect { all: bool },
    Except { all: bool },
    Minus { all: bool },
}

/// Core query without set operations
#[derive(Debug, Clone, PartialEq)]
pub enum QueryPrimary {
    Select(Loc<Select>),
    Values(Loc<Values>),
    Parenthesized(Box<Loc<Query>>),
}

// ------------- SELECT statement -------------

/// SELECT statement
#[derive(Debug, Clone, PartialEq)]
pub struct Select {
    pub distinct: SetQuantifier,
    pub projection: Loc<Projection>,
    pub from: Option<Loc<From>>,
    pub where_clause: Option<Loc<Where>>,
    pub group_by: Option<Loc<GroupBy>>,
    pub having: Option<Loc<Expr>>,
    pub window: Option<Loc<Window>>,
    pub qualify: Option<Loc<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SetQuantifier {
    All,
    Distinct,
    DistinctOn(DelimitedList<Loc<Expr>>),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Projection {
    pub list: DelimitedList<Loc<ProjectionItem>>,
}

impl Projection {
    pub fn items(&self) -> impl Iterator<Item = &Loc<ProjectionItem>> {
        self.list.items.iter()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionItem {
    pub expr: Loc<Expr>,
    pub alias: Option<Loc<Identifier>>,
}

// ------------- FROM clause -------------

#[derive(Debug, Clone, PartialEq)]
pub struct From {
    pub sources: DelimitedList<Loc<TableRef>>,
}

/// A table reference is a join tree over table factors
#[derive(Debug, Clone, PartialEq)]
pub enum TableRef {
    /// Leaf node (named table, subquery, or parenthesized group)
    Factor(Loc<TableFactor>),
    /// Binary join
    Join(Loc<Join>),
}

/// Table factors that can appear as leaves in the join tree
#[derive(Debug, Clone, PartialEq)]
pub enum TableFactor {
    Named(Loc<NamedTableFactor>),
    Function(Loc<FunctionTableFactor>),
    Subquery(Loc<SubqueryTableFactor>),
    Parenthesized(Box<Loc<TableRef>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedTableFactor {
    pub name: Loc<QualifiedName>,
    pub alias: Option<Loc<Identifier>>,
    pub lateral: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionTableFactor {
    pub name: Loc<QualifiedName>,
    pub args: DelimitedList<Loc<Expr>>,
    pub alias: Option<Loc<Identifier>>,
    pub columns: Option<DelimitedList<Loc<Identifier>>>,
    pub lateral: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubqueryTableFactor {
    pub query: Loc<Query>,
    pub alias: Option<Loc<Identifier>>,
    pub lateral: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Join {
    pub left: Box<Loc<TableRef>>,
    pub kind: JoinKind,
    pub right: Box<Loc<TableRef>>,
    pub constraint: Option<Loc<JoinConstraint>>,
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
    On(Loc<Expr>),
    Using(DelimitedList<Loc<Identifier>>),
}

// ------------- WHERE clause -------------

#[derive(Debug, Clone, PartialEq)]
pub struct Where {
    pub expr: Loc<Expr>,
}

// ------------- GROUP BY clause -------------

#[derive(Debug, Clone, PartialEq)]
pub struct GroupBy {
    pub items: DelimitedList<Loc<GroupByItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GroupByItem {
    Expr(Loc<Expr>),
    Rollup(AstVec<Loc<Expr>>),
    Cube(AstVec<Loc<Expr>>),
    GroupingSets(AstVec<Loc<GroupingSet>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GroupingSet {
    Expr(Loc<Expr>),
    Exprs(AstVec<Loc<Expr>>),
}

// ------------- WINDOW clause -------------

#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub windows: AstVec<Loc<WindowDef>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowDef {
    pub name: Identifier,
    pub spec: Loc<WindowSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowSpec {
    pub partition_by: Option<DelimitedList<Loc<Expr>>>,
    pub order_by: Option<Loc<OrderBy>>,
    pub frame: Option<Box<Loc<WindowFrame>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowFrame {
    pub unit: FrameUnit,
    pub start: Box<Loc<FrameBound>>,
    pub end: Option<Box<Loc<FrameBound>>>,
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
    Preceding(Box<Loc<Expr>>),
    CurrentRow,
    Following(Box<Loc<Expr>>),
    UnboundedFollowing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindowRef {
    Spec(Box<Loc<WindowSpec>>),
    Name(Loc<Identifier>),
}

// ------------- ORDER BY / LIMIT / OFFSET -------------

/// Query tail with ORDER BY, LIMIT, OFFSET (binds to the whole query
/// expression)
#[derive(Debug, Clone, PartialEq)]
pub struct QuerySuffix {
    pub order_by: Option<Loc<OrderBy>>,
    pub limit: Option<Loc<Limit>>,
    pub offset: Option<Loc<Offset>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderBy {
    pub items: DelimitedList<Loc<OrderByItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderByItem {
    pub expr: Loc<Expr>,
    pub direction: Option<SortDirection>,
    pub nulls: Option<NullsOrder>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullsOrder {
    First,
    Last,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Limit {
    pub count: Loc<Expr>,
    pub style: LimitKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Offset {
    pub count: Loc<Expr>,
    pub rows_keyword: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    FetchFirst,
    Limit,
}

// ------------- VALUES statement -------------

/// VALUES statement
#[derive(Debug, Clone, PartialEq)]
pub struct Values {
    pub rows: Vec<Loc<DelimitedList<Loc<Expr>>>>,
}

// ------------- Expressions -------------

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Name(Loc<QualifiedName>),
    Literal(Loc<Literal>),
    Binary(Loc<Binary>),
    Unary(Loc<Unary>),
    Paren(Loc<Paren>),
    Subquery(Box<Loc<Query>>),
    IsNull(Loc<IsNull>),
    Between(Loc<Between>),
    Like(Loc<Like>),
    ILike(Loc<ILike>),
    Similar(Loc<Similar>),
    FunctionCall(Loc<FunctionCall>),
    Array(DelimitedList<Loc<Expr>>),
    Quantified(Loc<Quantified>),
    Case(Loc<Case>),
    In(Loc<In>),
    Over(Loc<Over>),
    Exists(Box<Loc<Query>>),
    Cast(Loc<Cast>),
    Subscript(Loc<Subscript>),
    Row(Loc<Row>),
    AtTimeZone(Loc<AtTimeZone>),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cast {
    pub expr: Box<Loc<Expr>>,
    pub data_type: Loc<DataType>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Named(Loc<QualifiedName>),
    Parameterized {
        name: Loc<QualifiedName>,
        params: AstVec<Loc<TypeParam>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeParam {
    Number(i64),
    Ident(Identifier),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subscript {
    pub expr: Box<Loc<Expr>>,
    pub index: Box<Loc<Expr>>,
    pub upper: Option<Box<Loc<Expr>>>, // for slice: arr[1:3]
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub exprs: DelimitedList<Loc<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AtTimeZone {
    pub expr: Box<Loc<Expr>>,
    pub timezone: Box<Loc<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binary {
    pub left: Box<Loc<Expr>>,
    pub op: Option<Loc<OpTag>>,
    pub right: Option<Box<Loc<Expr>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unary {
    pub op_tok: Loc<OpTag>,
    pub expr: Box<Loc<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Paren {
    pub open: Identifier,
    pub expr: Box<Loc<Expr>>,
    pub close: Option<Identifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsNull {
    pub expr: Box<Loc<Expr>>,
    pub not: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Between {
    pub expr: Box<Loc<Expr>>,
    pub low: Box<Loc<Expr>>,
    pub high: Box<Loc<Expr>>,
    pub not: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Like {
    pub expr: Box<Loc<Expr>>,
    pub pattern: Box<Loc<Expr>>,
    pub not: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ILike {
    pub expr: Box<Loc<Expr>>,
    pub pattern: Box<Loc<Expr>>,
    pub not: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Similar {
    pub expr: Box<Loc<Expr>>,
    pub pattern: Box<Loc<Expr>>,
    pub escape: Option<Box<Loc<Expr>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    pub name: Loc<QualifiedName>,
    pub distinct: bool,
    pub args: Loc<DelimitedList<Loc<Expr>>>,
    pub filter: Option<Box<Loc<Expr>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Quantified {
    pub quantifier: Quantifier,
    pub expr: Box<Loc<Expr>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantifier {
    Any,
    Some,
    All,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub operand: Option<Box<Loc<Expr>>>,
    pub when_clauses: Vec<WhenClause>, // Vec for recursive Expr
    pub else_clause: Option<Box<Loc<Expr>>>,
}

/// WHEN ... THEN ... clause in a CASE expression
#[derive(Debug, Clone, PartialEq)]
pub struct WhenClause {
    pub when: Loc<Expr>,
    pub then: Loc<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct In {
    pub expr: Box<Loc<Expr>>,
    pub list: ExprList,
    pub not: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprList {
    Subquery(Box<Loc<Query>>),
    Exprs(Vec<Loc<Expr>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Over {
    pub name: Loc<QualifiedName>,
    pub args: Loc<DelimitedList<Loc<Expr>>>,
    pub over: WindowRef,
    pub filter: Option<Box<Loc<Expr>>>,
}

// ------------- Literals -------------

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(i64),
    Float(f64),
    String(Identifier),
    Null,
    Boolean(Boolean),
    TypedString {
        data_type: TypedLiteralKind,
        value: Identifier,
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
    Interval,
}

// ------------- Names and identifiers -------------

/// Name such as schema.table.column or simply column; supports STAR
#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedName {
    pub parts: DelimitedList<Loc<NamePart>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamePart {
    Ident(Identifier),
    Star,
}

// ------------- Utility containers -------------

/// A list of items separated by a separator
/// Uses Vec instead of SmallVec because it's used in recursive Expr types
#[derive(Debug, Clone, PartialEq)]
pub struct DelimitedList<T> {
    pub items: Vec<T>,
    pub seps: Vec<Loc<TokenKind>>,
}

impl<T> Default for DelimitedList<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            seps: Vec::new(),
        }
    }
}
