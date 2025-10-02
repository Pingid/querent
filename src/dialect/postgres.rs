use phf::phf_map;

use crate::{
    dialect::{CaseFold, CaseRules, CommentStyle, Dialect, DialectSpec},
    tokenize::{Assoc, Keyword, OpTag, Operator, QuoteStyle, op},
};

#[derive(Debug, Clone, Copy)]
pub struct PgDialect;

impl Default for PgDialect {
    fn default() -> Self {
        PgDialect
    }
}

impl Dialect for PgDialect {
    fn spec(&self) -> &DialectSpec {
        &PG_SPEC
    }
}

/// The global PG dialect spec — no runtime alloc, no cloning.
pub static PG_SPEC: DialectSpec = DialectSpec {
    keywords: &PG_KEYWORDS,
    operators: &PG_OPERATORS,
    quote_styles: &[QuoteStyle::Double],
    case_rules: CaseRules {
        keywords_case_insensitive: true,
        word_ops_case_insensitive: true,
        unquoted_identifier_fold: CaseFold::Lower, // PostgreSQL defaults to lowercase
        quoted_identifiers_case_sensitive: true,
    },
    comment_styles: &[CommentStyle::Line, CommentStyle::Block],
};

/// Compile-time perfect hash map for operators.
/// PgQL operators with precedence (higher number = higher precedence).
static PG_OPERATORS: phf::Map<&'static str, Operator> = phf_map! {
    // Arithmetic
    "^"  => op("^",  10, OpTag::Exp,  Assoc::Left),  // exponentiation
    "*"  => op("*",   9, OpTag::Mul,  Assoc::Left),
    "/"  => op("/",   9, OpTag::Div,  Assoc::Left),
    "%"  => op("%",   9, OpTag::Mod,  Assoc::Left),
    "+"  => op("+",   8, OpTag::Add,  Assoc::Left),
    "-"  => op("-",   8, OpTag::Sub,  Assoc::Left),


    "||" => op("||",  7, OpTag::Concat, Assoc::Left),

    // Regex
    "~"   => op("~",  7, OpTag::Regex,     Assoc::None),
    "!~"  => op("!~", 7, OpTag::NotRegex,  Assoc::None),
    "~*"  => op("~*", 7, OpTag::RegexI,    Assoc::None),
    "!~*" => op("!~*",7, OpTag::NotRegexI, Assoc::None),

    // Bitwise & shifts
    "&"  => op("&",   7, OpTag::BitAnd, Assoc::Left),
    "|"  => op("|",   7, OpTag::BitOr,  Assoc::Left),
    "#"  => op("#",   7, OpTag::BitXor, Assoc::Left),
    "<<" => op("<<",  7, OpTag::Shl,    Assoc::Left),
    ">>" => op(">>",  7, OpTag::Shr,    Assoc::Left),

    // Containment / overlap (arrays, ranges, hstore, jsonb etc.)
    "@>" => op("@>",  7, OpTag::Contains,     Assoc::None),
    "<@" => op("<@",  7, OpTag::ContainedBy,  Assoc::None),
    "&&" => op("&&",  7, OpTag::Overlap,      Assoc::None),
    // JSON/JSONB
    "->"  => op("->",  7, OpTag::JsonGet,     Assoc::Left),
    "->>" => op("->>", 7, OpTag::JsonGetText, Assoc::Left),
    "#>"  => op("#>",  7, OpTag::JsonPath,    Assoc::Left),
    "#>>" => op("#>>", 7, OpTag::JsonPathText,Assoc::Left),
    "?"   => op("?",   7, OpTag::JsonKeyExists, Assoc::None),
    "?|"  => op("?|",  7, OpTag::JsonAnyKey,    Assoc::None),
    "?&"  => op("?&",  7, OpTag::JsonAllKeys,   Assoc::None),
    "@?"  => op("@?",  7, OpTag::JsonPathMatch, Assoc::None),
    "@@"  => op("@@",  7, OpTag::JsonPathBool,  Assoc::None),


    // Comparisons
    "="  => op("=",   5, OpTag::Eq,  Assoc::None),
    "<>" => op("<>",  5, OpTag::Neq, Assoc::None),
    "!=" => op("!=",  5, OpTag::Neq, Assoc::None), // alias
    "<"  => op("<",   5, OpTag::Lt,  Assoc::None),
    "<=" => op("<=",  5, OpTag::Lte, Assoc::None),
    ">"  => op(">",   5, OpTag::Gt,  Assoc::None),
    ">=" => op(">=",  5, OpTag::Gte, Assoc::None),

    // Logical
    "AND" => op("AND", 2, OpTag::And, Assoc::Left),
    "OR"  => op("OR",  1, OpTag::Or,  Assoc::Left),
};

/// PgQL keywords (reserved + common non-reserved) you’ll likely want to tokenize.
static PG_KEYWORDS: phf::Map<&'static str, Keyword> = phf_map! {
   // Query core
   "SELECT" => Keyword::Select,
   "DISTINCT" => Keyword::Distinct,
   "ALL" => Keyword::All,
   "FROM" => Keyword::From,
   "WHERE" => Keyword::Where,
   "GROUP" => Keyword::Group,
   "BY" => Keyword::By,
   "HAVING" => Keyword::Having,
   "ORDER" => Keyword::Order,
   "ASC" => Keyword::Asc,
   "DESC" => Keyword::Desc,
   "NULLS" => Keyword::Nulls,
   "FIRST" => Keyword::First,
   "LAST" => Keyword::Last,

   // Set ops
   "UNION" => Keyword::Union,
   "INTERSECT" => Keyword::Intersect,
   "EXCEPT" => Keyword::Except,

   // CTEs / subqueries
   "WITH" => Keyword::With,
   "RECURSIVE" => Keyword::Recursive,
   "AS" => Keyword::As,
   "LATERAL" => Keyword::Lateral,

   // Joins
   "JOIN" => Keyword::Join,
   "LEFT" => Keyword::Left,
   "RIGHT" => Keyword::Right,
   "FULL" => Keyword::Full,
   "INNER" => Keyword::Inner,
   "OUTER" => Keyword::Outer,
   "CROSS" => Keyword::Cross,
   "NATURAL" => Keyword::Natural,
   "ON" => Keyword::On,
   "USING" => Keyword::Using,

   // Predicates / matching
   "NOT" => Keyword::Not,
   "IS" => Keyword::Is,
   "NULL" => Keyword::Null,
   "TRUE" => Keyword::True,
   "FALSE" => Keyword::False,
   "UNKNOWN" => Keyword::Unknown,
   "BETWEEN" => Keyword::Between,
   "IN" => Keyword::In,
   "LIKE" => Keyword::Like,
   "ILIKE" => Keyword::ILike,
   "ARRAY" => Keyword::Array,
   "ESCAPE" => Keyword::Escape,
   "SIMILAR" => Keyword::Similar,
   "TO" => Keyword::To,
   "EXISTS" => Keyword::Exists,
   "ANY" => Keyword::Any,
   "SOME" => Keyword::Some,
   "OVERLAPS" => Keyword::Overlaps,

   // Case expressions
   "CASE" => Keyword::Case,
   "WHEN" => Keyword::When,
   "THEN" => Keyword::Then,
   "ELSE" => Keyword::Else,
   "END" => Keyword::End,

   // Windows
   "OVER" => Keyword::Over,
   "PARTITION" => Keyword::Partition,
   "RANGE" => Keyword::Range,
   "ROWS" => Keyword::Rows,
   "UNBOUNDED" => Keyword::Unbounded,
   "PRECEDING" => Keyword::Preceding,
   "FOLLOWING" => Keyword::Following,
   "CURRENT" => Keyword::Current,
   "ROW" => Keyword::Row,
   "WINDOW" => Keyword::Window,
   "FILTER" => Keyword::Filter,

   // Pagination
   "LIMIT" => Keyword::Limit,
   "OFFSET" => Keyword::Offset,
   "FETCH" => Keyword::Fetch,
   "NEXT" => Keyword::Next,
   "ONLY" => Keyword::Only,

   // DML
   "INSERT" => Keyword::Insert,
   "INTO" => Keyword::Into,
   "VALUES" => Keyword::Values,
   "UPDATE" => Keyword::Update,
   "SET" => Keyword::Set,
   "DELETE" => Keyword::Delete,
   "RETURNING" => Keyword::Returning,

   // Upsert / merge-ish
   "CONFLICT" => Keyword::Conflict,
   "DO" => Keyword::Do,
   "NOTHING" => Keyword::Nothing,

   // DDL
   "CREATE" => Keyword::Create,
   "ALTER" => Keyword::Alter,
   "DROP" => Keyword::Drop,
   "TABLE" => Keyword::Table,
   "VIEW" => Keyword::View,
   "MATERIALIZED" => Keyword::Materialized,
   "SCHEMA" => Keyword::Schema,
   "INDEX" => Keyword::Index,
   "UNIQUE" => Keyword::Unique,
   "CONCURRENTLY" => Keyword::Concurrently,
   "CONSTRAINT" => Keyword::Constraint,
   "PRIMARY" => Keyword::Primary,
   "FOREIGN" => Keyword::Foreign,
   "KEY" => Keyword::Key,
   "REFERENCES" => Keyword::References,
   "DEFERRABLE" => Keyword::Deferrable,
   "INITIALLY" => Keyword::Initially,
   "DEFERRED" => Keyword::Deferred,
   "IMMEDIATE" => Keyword::Immediate,
   "CHECK" => Keyword::Check,
   "DEFAULT" => Keyword::Default,
   "COLLATE" => Keyword::Collate,

   // Types & casts
   "CAST" => Keyword::Cast,
   "COALESCE" => Keyword::Coalesce,
   "NULLIF" => Keyword::NullIf,
   "INTERVAL" => Keyword::Interval,
   "DATE" => Keyword::Date,
   "TIME" => Keyword::Time,
   "TIMESTAMP" => Keyword::Timestamp,
   "WITHOUT" => Keyword::Without,
   "ZONE" => Keyword::Zone,

   // JSON/JSONB (operators are tokens above; include common funcs/clauses as keywords when needed)
   "JSONB" => Keyword::Jsonb,
   "JSON" => Keyword::Json,
};
