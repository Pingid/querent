use crate::span::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords & identifiers
    Keyword(Keyword),
    Identifier,
    IdentifierQuoted(QuoteStyle),

    // Literals
    Float,
    Number,
    Str,

    // Operators
    Operator(Operator),

    // Punctuation / delimiters
    Dot,
    Comma,
    Semicolon,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,

    // Control / misc
    Eof,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    // ================ ANSI Core Keywords ================

    // Query structure
    Select,
    Distinct,
    From,
    Where,
    With,
    Union,
    Intersect,
    Except,
    Minus, // SQL:1999 optional (Oracle)

    // Set quantifiers
    All,
    Any,
    Some,

    // DML
    Insert,
    Update,
    Delete,
    Values,
    Into,
    Merge,
    Matched,

    // Joins
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

    // Grouping / aggregation
    Group,
    Having,
    Order,
    By,
    Asc,
    Desc,
    Nulls,
    First,
    Last,
    Rollup,
    Cube,
    Grouping,
    Set,

    // Predicates & logic
    Overlaps,

    // Null / boolean literals
    Null,
    True,
    False,
    Unknown,

    // Case expressions
    Case,
    When,
    Then,
    Else,
    End,

    // Windowing & analytic
    Over,
    Partition,
    Window,
    Filter,

    // Window frame
    Preceding,
    Following,
    Current,
    Row,
    Rows,
    Range,
    Unbounded,

    // Fetch / offset / pagination
    Offset,
    Fetch,
    Only,
    Next,
    Limit,

    // Temporal
    Date,
    Time,
    Timestamp,
    Interval,

    // Functions / casts
    Cast,
    Coalesce,
    Nullif,

    // DDL
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

    // Misc
    Recursive,
    As,
    Escape,
    To,
    Array,
    Without,
    Zone,

    // ================ Postgres-Specific Keywords ================

    // Predicates
    Ilike,

    // DML extensions
    Returning,
    Conflict,
    Do,
    Nothing,

    // Schema / object management
    Materialized,
    Index,
    Concurrently,

    // Constraints / transactions
    Deferrable,
    Initially,
    Deferred,
    Immediate,

    // JSON / JSONB types
    Jsonb,
    Json,
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
    pub precedence: u8,
    pub assoc: Assoc,
    pub semantic_tag: OpTag,
    pub fixity: Fixity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fixity {
    Prefix,
    Infix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assoc {
    Left,
    Right,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpTag {
    // ================ ANSI Core Operators ================

    // String concatenation
    Concat,

    // Arithmetic
    Add,
    UnaryPlus,
    UnaryMinus,
    Sub,
    Mul,
    Div,
    Mod,
    Exp,

    // Logical
    And,
    Or,
    Not,
    Exists,

    // Comparisons & predicates
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    Is,
    In,
    Between,
    Like,
    Similar,
    Overlaps,

    // ================ Postgres-Specific Operators ================

    // Case-insensitive LIKE
    Ilike,

    // Regex match ops
    Regex,
    RegexI,
    NotRegex,
    NotRegexI,

    // JSON / JSONB
    JsonArrow,     // ->   (alias: JsonGet)
    JsonArrowText, // ->>  (alias: JsonGetText)
    JsonPath,      // #>
    JsonPathText,  // #>>
    JsonGet,       // redundant tag if you want to distinguish
    JsonGetText,
    JsonKeyExists, // ?
    JsonAnyKey,    // ?|
    JsonAllKeys,   // ?&
    JsonPathMatch, // @?
    JsonPathBool,  // @@

    // Bitwise & shifts
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,

    // Range / array containment & overlap
    Contains,    // @>
    ContainedBy, // <@
    Overlap,     // &&
}

// ---------------- Helper functions ----------------
impl Operator {
    pub const fn new(precedence: u8, semantic_tag: OpTag, assoc: Assoc, fixity: Fixity) -> Self {
        Self {
            precedence,
            assoc,
            semantic_tag,
            fixity,
        }
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

    pub fn strip_quotes<'txt>(&self, text: &'txt str) -> &'txt str {
        match self {
            QuoteStyle::Double => text.trim_matches('"'),
            QuoteStyle::Backtick => text.trim_matches('`'),
            QuoteStyle::Bracket => text.trim_matches('[').trim_matches(']'),
        }
    }
}
