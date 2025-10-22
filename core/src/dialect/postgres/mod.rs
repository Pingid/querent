use crate::{
    dialect::{CaseFold, CommentStyle, DialectSpec, StyleRules, ansi},
    lex::QuoteStyle,
};

mod keyword;
use keyword::KEYWORDS;

mod operator;
use operator::OP_TABLE;

mod queries;
pub use queries::*;

#[derive(Debug, Clone, Copy)]
pub struct Postgres {
    pub spec: &'static DialectSpec,
}

impl Default for Postgres {
    fn default() -> Self {
        Self { spec: &SPEC }
    }
}

/// The global PG dialect spec — no runtime alloc, no cloning.
pub static SPEC: DialectSpec = DialectSpec {
    name: "postgres",
    keywords: &KEYWORDS,
    operators: &OP_TABLE,
    style_rules: StyleRules {
        keywords_case_insensitive: true,
        word_ops_case_insensitive: true,
        unquoted_identifier_fold: CaseFold::Lower, // PostgreSQL defaults to lowercase
        quoted_identifiers_case_sensitive: true,
        comments: &[CommentStyle::DoubleDash, CommentStyle::SlashStar],
        quotes: &[QuoteStyle::Double],
    },
    follow_rules: &[ansi::RULES],
};
