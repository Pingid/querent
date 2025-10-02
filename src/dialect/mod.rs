use crate::tokenize::{Keyword, Operator, QuoteStyle};

pub mod ansi;
pub mod postgres;

pub trait Dialect {
    fn spec(&self) -> &DialectSpec;
}

#[derive(Debug, Clone, Copy)]
pub struct DialectSpec {
    pub keywords: &'static phf::Map<&'static str, Keyword>,
    pub operators: &'static phf::Map<&'static str, Operator>,
    pub quote_styles: &'static [QuoteStyle],
    pub case_rules: CaseRules,
    pub comment_styles: &'static [CommentStyle],
}

impl DialectSpec {
    pub fn match_keyword(&self, keyword: &str) -> Option<Keyword> {
        let kw_lookup = match self.case_rules.keywords_case_insensitive {
            true => std::borrow::Cow::Owned(keyword.to_ascii_uppercase()),
            false => std::borrow::Cow::Borrowed(keyword),
        };
        self.keywords.get(&kw_lookup).copied()
    }

    pub fn match_operator(&self, operator: &str) -> Option<Operator> {
        let op_lookup = match self.case_rules.word_ops_case_insensitive {
            true => std::borrow::Cow::Owned(operator.to_ascii_uppercase()),
            false => std::borrow::Cow::Borrowed(operator),
        };
        self.operators.get(&op_lookup).copied()
    }

    pub fn supports_quote_style(&self, quote: QuoteStyle) -> bool {
        self.quote_styles.contains(&quote)
    }

    pub fn supports_comment_style(&self, comment: CommentStyle) -> bool {
        self.comment_styles.contains(&comment)
    }

    pub fn is_ident_start(ch: char) -> bool {
        ch == '_' || ch == '$' || ch.is_alphabetic()
    }

    pub fn is_ident_continue(ch: char) -> bool {
        ch == '_' || ch == '$' || ch.is_alphanumeric()
    }

    pub fn max_op_len(&self) -> usize {
        self.operators.keys().map(|k| k.len()).max().unwrap_or(0)
    }

    /// Fold an *unquoted* identifier according to dialect rules.
    pub fn fold_unquoted_identifier<'a>(&self, ident: &'a str) -> std::borrow::Cow<'a, str> {
        match self.case_rules.unquoted_identifier_fold {
            CaseFold::Upper => std::borrow::Cow::Owned(ident.to_ascii_uppercase()),
            CaseFold::Lower => std::borrow::Cow::Owned(ident.to_ascii_lowercase()),
            CaseFold::None => std::borrow::Cow::Borrowed(ident),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseFold {
    Upper,
    Lower,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaseRules {
    /// Are keywords case-insensitive? (ANSI & all major engines: yes)
    pub keywords_case_insensitive: bool,
    /// Are word-operators (AND/OR/NOT/LIKE/ILIKE/IN, etc.) case-insensitive?
    pub word_ops_case_insensitive: bool,
    /// How to fold *unquoted* identifiers (tables/cols): Upper (ANSI/Oracle),
    /// Lower (Postgres), or None (e.g., user/OS/collation-driven).
    pub unquoted_identifier_fold: CaseFold,
    /// Quoted identifiers are treated as case-sensitive (ANSI default).
    pub quoted_identifiers_case_sensitive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentStyle {
    Line,
    Block,
    Hash,
}
