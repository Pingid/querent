use crate::{
    dialect::{CaseFold, CommentStyle, DialectSpec, StyleRules, ansi},
    lex::QuoteStyle,
};

mod keyword;
use keyword::KEYWORDS;

mod operator;
use operator::OPERATORS;

mod queries;
pub use queries::*;

/// The global SQLite dialect spec — no runtime alloc, no cloning.
pub static SPEC: DialectSpec = DialectSpec {
    name: "sqlite",
    keywords: &KEYWORDS,
    operators: &OPERATORS,
    functions: &ansi::FUNCTIONS,
    style_rules: StyleRules {
        keywords_case_insensitive: true,
        word_ops_case_insensitive: true,
        unquoted_identifier_fold: CaseFold::Preserve, // SQLite preserves case
        quoted_identifiers_case_sensitive: true,
        comments: &[CommentStyle::DoubleDash, CommentStyle::SlashStar],
        quotes: &[QuoteStyle::Double],
    },
    follow_rules: &[ansi::RULES],
};
