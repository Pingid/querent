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
    "<=" => op("<=", 5, OpTag::Lte, Assoc::Left),
    ">=" => op(">=", 5, OpTag::Gte, Assoc::Left),
    "!=" => op("!=", 5, OpTag::Neq, Assoc::Left),
    "<>" => op("<>", 5, OpTag::Neq, Assoc::Left),
    "="  => op("=",  5, OpTag::Eq,  Assoc::Left),
    "<"  => op("<",  5, OpTag::Lt,  Assoc::Left),
    ">"  => op(">",  5, OpTag::Gt,  Assoc::Left),
    "+"  => op("+",  6, OpTag::Add, Assoc::Left),
    "-"  => op("-",  6, OpTag::Sub, Assoc::Left),
    "*"  => op("*",  7, OpTag::Mul, Assoc::Left),
    "/"  => op("/",  7, OpTag::Div, Assoc::Left),
    "%"  => op("%",  7, OpTag::Mod, Assoc::Left),
    "AND" => op("AND", 2, OpTag::And, Assoc::Left),
    "OR"  => op("OR",  1, OpTag::Or,  Assoc::Left),
    "LIKE" => op("LIKE", 3, OpTag::Like, Assoc::Left),
};

/// Compile-time perfect hash map for keywords.
static ANSI_KEYWORDS: phf::Map<&'static str, Keyword> = phf_map! {
    "SELECT"    => Keyword::Select,
    "DISTINCT"  => Keyword::Distinct,
    "FROM"      => Keyword::From,
    "WHERE"     => Keyword::Where,
    "WITH"      => Keyword::With,
    "RECURSIVE" => Keyword::Recursive,
    "AS"        => Keyword::As,
    "UNION"     => Keyword::Union,
    "ALL"       => Keyword::All,

    "INSERT"    => Keyword::Insert,
    "UPDATE"    => Keyword::Update,
    "DELETE"    => Keyword::Delete,
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
    "GROUP"     => Keyword::Group,
    "HAVING"    => Keyword::Having,
    "ORDER"     => Keyword::Order,
    "BY"        => Keyword::By,
    "ASC"       => Keyword::Asc,
    "DESC"      => Keyword::Desc,
    "NOT"       => Keyword::Not,
    "IS"        => Keyword::Is,
    "BETWEEN"   => Keyword::Between,
    "NULL"      => Keyword::Null,
    "ROLLUP"    => Keyword::Rollup,
    "CUBE"      => Keyword::Cube,
    "GROUPING"  => Keyword::Grouping,
    "SETS"      => Keyword::Sets,
};
