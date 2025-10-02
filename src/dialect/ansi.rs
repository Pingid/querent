use phf::phf_map;

use crate::{
    dialect::{CaseFold, CaseRules, CommentStyle, Dialect, DialectSpec},
    tokenize::{Assoc, Keyword, OpTag, Operator, QuoteStyle, op},
};

#[derive(Debug, Clone, Copy)]
pub struct AnsiDialect;

impl Default for AnsiDialect {
    fn default() -> Self {
        AnsiDialect
    }
}

impl Dialect for AnsiDialect {
    fn spec(&self) -> &DialectSpec {
        &ANSI_SPEC
    }
}

/// The global ANSI dialect spec — no runtime alloc, no cloning.
pub static ANSI_SPEC: DialectSpec = DialectSpec {
    keywords: &ANSI_KEYWORDS,
    operators: &ANSI_OPERATORS,
    quote_styles: &[QuoteStyle::Double],
    case_rules: CaseRules {
        keywords_case_insensitive: true,
        word_ops_case_insensitive: true,
        unquoted_identifier_fold: CaseFold::Upper,
        quoted_identifiers_case_sensitive: true,
    },
    comment_styles: &[CommentStyle::Line, CommentStyle::Block],
};

/// Compile-time perfect hash map for operators.
static ANSI_OPERATORS: phf::Map<&'static str, Operator> = phf_map! {
    // Concatenation
    "||" => op("||", 6, OpTag::Concat, Assoc::Left),

    // Arithmetic
    "*"  => op("*",  7, OpTag::Mul, Assoc::Left),
    "/"  => op("/",  7, OpTag::Div, Assoc::Left),
    "+"  => op("+",  6, OpTag::Add, Assoc::Left),
    "-"  => op("-",  6, OpTag::Sub, Assoc::Left),

    // Comparisons & predicates (same precedence tier)
    "="  => op("=",  4, OpTag::Eq,  Assoc::None),
    "<>" => op("<>", 4, OpTag::Neq, Assoc::None),
    "!=" => op("!=", 4, OpTag::Neq, Assoc::None),
    "<"  => op("<",  4, OpTag::Lt,  Assoc::None),
    "<=" => op("<=", 4, OpTag::Lte, Assoc::None),
    ">"  => op(">",  4, OpTag::Gt,  Assoc::None),
    ">=" => op(">=", 4, OpTag::Gte, Assoc::None),

    // Logical
    "AND" => op("AND", 2, OpTag::And, Assoc::Left),
    "OR"  => op("OR",  1, OpTag::Or,  Assoc::Left),
};

/// Compile-time perfect hash map for keywords.
static ANSI_KEYWORDS: phf::Map<&'static str, Keyword> = phf_map! {
    // Query
    "SELECT"    => Keyword::Select,
    "ALL"       => Keyword::All,
    "FROM"      => Keyword::From,
    "WHERE"     => Keyword::Where,
    "GROUP"     => Keyword::Group,
    "BY"        => Keyword::By,
    "HAVING"    => Keyword::Having,
    "ORDER"     => Keyword::Order,
    "ASC"       => Keyword::Asc,
    "DESC"      => Keyword::Desc,
    "NULLS"     => Keyword::Nulls,
    "LAST"      => Keyword::Last,

    // Set operations
    "UNION"     => Keyword::Union,
    "INTERSECT" => Keyword::Intersect,
    "EXCEPT"    => Keyword::Except,

    // CTEs
    "WITH"      => Keyword::With,
    "RECURSIVE" => Keyword::Recursive,
    "AS"        => Keyword::As,

    // Joins
    "JOIN"      => Keyword::Join,
    "LEFT"      => Keyword::Left,
    "RIGHT"     => Keyword::Right,
    "FULL"      => Keyword::Full,
    "INNER"     => Keyword::Inner,
    "OUTER"     => Keyword::Outer,
    "CROSS"     => Keyword::Cross,
    "NATURAL"   => Keyword::Natural,
    "ON"        => Keyword::On,
    "USING"     => Keyword::Using,

    // Predicates / logical
    "NOT"               => Keyword::Not,
    "IS"                => Keyword::Is,
    "CASE"              => Keyword::Case,
    "WHEN"              => Keyword::When,
    "THEN"              => Keyword::Then,
    "ELSE"              => Keyword::Else,
    "END"               => Keyword::End,
    "NULL"              => Keyword::Null,
    "TRUE"              => Keyword::True,
    "FALSE"             => Keyword::False,
    "UNKNOWN"           => Keyword::Unknown,
    "BETWEEN"           => Keyword::Between,
    "IN"                => Keyword::In,
    "LIKE"              => Keyword::Like,
    "ARRAY"            => Keyword::Array,
    "ESCAPE"            => Keyword::Escape,
    "SIMILAR"           => Keyword::Similar, // used with TO
    "TO"                => Keyword::To,
    "EXISTS"            => Keyword::Exists,
    "ANY"               => Keyword::Any,
    "SOME"              => Keyword::Some,
    "OVERLAPS"          => Keyword::Overlaps,
    "DISTINCT"          => Keyword::Distinct, // (already above; keep once in your code)

    // Window functions
    "OVER"              => Keyword::Over,
    "PARTITION"         => Keyword::Partition,
    "RANGE"             => Keyword::Range,
    "ROWS"              => Keyword::Rows,
    "UNBOUNDED"         => Keyword::Unbounded,
    "PRECEDING"         => Keyword::Preceding,
    "FOLLOWING"         => Keyword::Following,
    "CURRENT"           => Keyword::Current,
    "ROW"               => Keyword::Row,
    "WINDOW"            => Keyword::Window,
    "FILTER"            => Keyword::Filter,

    // Pagination (SQL:2008)
    "OFFSET"            => Keyword::Offset,
    "FETCH"             => Keyword::Fetch,
    "FIRST"             => Keyword::First,   // reused above (ensure single enum variant)
    "NEXT"              => Keyword::Next,
    "ONLY"              => Keyword::Only,

    // DML
    "INSERT"            => Keyword::Insert,
    "INTO"              => Keyword::Into,
    "VALUES"            => Keyword::Values,
    "UPDATE"            => Keyword::Update,
    "SET"               => Keyword::Set,
    "DELETE"            => Keyword::Delete,

    // MERGE (SQL:2003)
    "MERGE"             => Keyword::Merge,
    "MATCHED"           => Keyword::Matched,

    // DDL (core)
    "CREATE"            => Keyword::Create,
    "ALTER"             => Keyword::Alter,
    "DROP"              => Keyword::Drop,
    "TABLE"             => Keyword::Table,
    "VIEW"              => Keyword::View,
    "SCHEMA"            => Keyword::Schema,
    "COLUMN"            => Keyword::Column,
    "ADD"               => Keyword::Add,
    "CONSTRAINT"        => Keyword::Constraint,
    "PRIMARY"           => Keyword::Primary,
    "FOREIGN"           => Keyword::Foreign,
    "KEY"               => Keyword::Key,
    "REFERENCES"        => Keyword::References,
    "UNIQUE"            => Keyword::Unique,
    "CHECK"             => Keyword::Check,
    "DEFAULT"           => Keyword::Default,
    "COLLATE"           => Keyword::Collate,

    // Data types & datetime
    "CAST"              => Keyword::Cast,
    "COALESCE"          => Keyword::Coalesce,
    "NULLIF"            => Keyword::NullIf,
    "INTERVAL"          => Keyword::Interval,
    "DATE"              => Keyword::Date,
    "TIME"              => Keyword::Time,
    "TIMESTAMP"         => Keyword::Timestamp,
    "WITHOUT"           => Keyword::Without,
    "ZONE"              => Keyword::Zone,
};
