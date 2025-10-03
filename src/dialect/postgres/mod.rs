use crate::{
    dialect::{CaseFold, CaseRules, CommentStyle, Dialect, DialectSpec},
    token::QuoteStyle,
};

// Include the generated Postgres keywords
mod keyword;
use keyword::KEYWORDS;

mod operator;
use operator::OP_TABLE;

#[derive(Debug, Clone, Copy, Default)]
pub struct Postgres;

impl Dialect for Postgres {
    fn get_spec(&self) -> &DialectSpec {
        &SPEC
    }
}

/// The global PG dialect spec — no runtime alloc, no cloning.
pub static SPEC: DialectSpec = DialectSpec {
    keywords: &KEYWORDS,
    operators: &OP_TABLE,
    quote_styles: &[QuoteStyle::Double],
    case_rules: CaseRules {
        keywords_case_insensitive: true,
        word_ops_case_insensitive: true,
        unquoted_identifier_fold: CaseFold::Lower, // PostgreSQL defaults to lowercase
        quoted_identifiers_case_sensitive: true,
    },
    comment_styles: &[CommentStyle::DoubleDash, CommentStyle::SlashStar],
    follow_keywords: &[],
};
