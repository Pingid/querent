use crate::{
    dialect::{CaseFold, CommentStyle, DialectSpec, StyleRules, ansi::RULES},
    lex::QuoteStyle,
};

mod keyword;
use keyword::KEYWORDS;

mod operator;
use operator::OP_TABLE;

mod queries;
pub use queries::*;

/// The global SQLite dialect spec — no runtime alloc, no cloning.
pub static SPEC: DialectSpec = DialectSpec {
    name: "sqlite",
    keywords: &KEYWORDS,
    operators: &OP_TABLE,
    style_rules: StyleRules {
        keywords_case_insensitive: true,
        word_ops_case_insensitive: true,
        unquoted_identifier_fold: CaseFold::Preserve, // SQLite preserves case
        quoted_identifiers_case_sensitive: true,
        comments: &[CommentStyle::DoubleDash, CommentStyle::SlashStar],
        quotes: &[QuoteStyle::Double],
    },
    follow_rules: &[RULES],
};
