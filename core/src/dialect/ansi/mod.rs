use crate::dialect::CaseFold;
use crate::dialect::CommentStyle;
use crate::dialect::DialectSpec;
use crate::dialect::StyleRules;
use crate::lex::QuoteStyle;

mod functions;
mod keyword;
mod operator;
mod rule;

pub(crate) use functions::FUNCTIONS;
use keyword::KEYWORDS;
use operator::OPERATORS;
pub(crate) use rule::RULES;

/// The global ANSI dialect spec — no runtime alloc, no cloning.
pub static SPEC: DialectSpec = DialectSpec {
    name: "ansi",
    keywords: &KEYWORDS,
    operators: &OPERATORS,
    functions: &FUNCTIONS,
    style_rules: StyleRules {
        keywords_case_insensitive: true,
        word_ops_case_insensitive: true,
        unquoted_identifier_fold: CaseFold::Upper,
        quoted_identifiers_case_sensitive: true,
        comments: &[CommentStyle::DoubleDash, CommentStyle::SlashStar],
        quotes: &[QuoteStyle::Double],
    },
    follow_rules: &[RULES],
};
