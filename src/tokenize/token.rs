use crate::tokenize::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Keyword(Keyword),
    Identifier,
    IdentifierQuoted(QuoteStyle),
    Float,
    Number,
    Str,
    Dot,
    Comma,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Semicolon,
    Eof,
    Operator(Operator),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    Select,
    Distinct,
    From,
    Where,
    With,
    Limit,
    Recursive,
    As,
    Union,
    Intersect,
    Except,
    Minus,
    All,
    Insert,
    Update,
    Delete,
    Join,
    Left,
    Right,
    Full,
    Inner,
    Outer,
    Cross,
    Natural,
    Lateral,
    On,
    Using,
    Group,
    Having,
    Order,
    By,
    Asc,
    Desc,
    Nulls,
    First,
    Last,
    Not,
    Is,
    Between,
    Like,
    Null,
    Rollup,
    Cube,
    Grouping,
    Sets,
    Case,
    When,
    Then,
    Else,
    End,
    In,
    Date,
    Time,
    Timestamp,
    Offset,
    Rows,
    Fetch,
    Only,
    Over,
    Partition,
    Preceding,
    Current,
    Row,
    Following,
    Unbounded,
    Range,
    And,
    Or,
    True,
    False,
    Unknown,
    Escape,
    Similar,
    To,
    Array,
    Exists,
    Any,
    Some,
    Overlaps,
    Window,
    Filter,
    Next,
    Into,
    Values,
    Set,
    Merge,
    Matched,
    Create,
    Alter,
    Drop,
    Table,
    View,
    Schema,
    Column,
    Add,
    Constraint,
    Primary,
    Foreign,
    Key,
    References,
    Unique,
    Check,
    Default,
    Collate,
    Cast,
    Coalesce,
    NullIf,
    Interval,
    Without,
    Zone,
    ILike,
    Returning,
    Conflict,
    Do,
    Nothing,
    Materialized,
    Index,
    Concurrently,
    Deferrable,
    Initially,
    Deferred,
    Immediate,
    Jsonb,
    Json,
    Custom(&'static str),
}

/// Quoted identifier styles supported by dialects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteStyle {
    Double,   // "col"
    Backtick, // `col`
    Bracket,  // [col]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Operator {
    pub symbol: &'static str, // textual token, e.g. "+", "->>", "<=>"
    pub precedence: u8,
    pub assoc: Assoc,
    pub semantic_tag: OpTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assoc {
    Left,
    Right,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpTag {
    Concat,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Not,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    Like,
    ILike,
    Similar,
    In,
    Between,
    Overlaps,
    Regex,
    RegexI,
    NotRegex,
    NotRegexI,
    JsonArrow,
    JsonArrowText,
    JsonPath,
    JsonPathText,
    JsonGet,
    JsonGetText,
    JsonKeyExists,
    JsonAnyKey,
    JsonAllKeys,
    JsonPathMatch,
    JsonPathBool,
    Spaceship,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Exp,
    Contains,
    ContainedBy,
    Overlap,
    Custom(&'static str),
}

// ---------------- Helper functions ----------------
pub const fn op(
    symbol: &'static str,
    precedence: u8,
    semantic_tag: OpTag,
    assoc: Assoc,
) -> Operator {
    Operator {
        symbol,
        precedence,
        assoc,
        semantic_tag,
    }
}

impl QuoteStyle {
    pub fn from_open_char(value: char) -> Option<Self> {
        match value {
            '"' => Some(QuoteStyle::Double),
            '`' => Some(QuoteStyle::Backtick),
            '[' => Some(QuoteStyle::Bracket),
            _ => None,
        }
    }

    pub fn open_char(self) -> char {
        match self {
            QuoteStyle::Double => '"',
            QuoteStyle::Backtick => '`',
            QuoteStyle::Bracket => '[',
        }
    }

    pub fn close_char(self) -> char {
        match self {
            QuoteStyle::Double => '"',
            QuoteStyle::Backtick => '`',
            QuoteStyle::Bracket => ']',
        }
    }
}
