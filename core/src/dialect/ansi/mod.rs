use crate::dialect::CaseFold;
use crate::dialect::CommentStyle;
use crate::dialect::DialectSpec;
use crate::dialect::StyleRules;
use crate::lex::QuoteStyle;

mod keyword;
use keyword::KEYWORDS;

mod operator;
use operator::OPERATORS;

mod functions;
pub(crate) use functions::FUNCTIONS;

mod rule;
pub use rule::RULES;

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
