use crate::dialect::CaseFold;
use crate::dialect::CommentStyle;
use crate::dialect::DialectSpec;
use crate::dialect::StyleRules;
use crate::dialect::ansi;
use crate::lex::QuoteStyle;

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
    reserved: crate::dialect::RESERVED,
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
