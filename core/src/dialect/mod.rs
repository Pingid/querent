use crate::lex::{Keyword, Operator, QuoteStyle, TokenKind};

mod follow;
pub use follow::*;

mod ansi;
pub use ansi::Ansi;

mod postgres;
pub use postgres::Postgres;

pub trait DialectSpecProvider {
    fn get_spec(&self) -> &'static DialectSpec;
}

#[derive(Debug, Clone)]
pub enum DialectKind {
    Ansi(ansi::Ansi),
    Postgres(postgres::Postgres),
}

impl Default for DialectKind {
    fn default() -> Self {
        DialectKind::Ansi(ansi::Ansi::default())
    }
}

impl DialectKind {
    pub fn name(&self) -> &'static str {
        match self {
            DialectKind::Ansi(_) => "ansi",
            DialectKind::Postgres(_) => "postgres",
        }
    }
}

impl DialectSpecProvider for DialectKind {
    fn get_spec(&self) -> &'static DialectSpec {
        match self {
            DialectKind::Ansi(d) => d.get_spec(),
            DialectKind::Postgres(d) => d.get_spec(),
        }
    }
}

impl From<&str> for DialectKind {
    fn from(s: &str) -> Self {
        match s {
            "postgres" | "pg" => DialectKind::Postgres(postgres::Postgres::default()),
            _ => DialectKind::Ansi(ansi::Ansi::default()),
        }
    }
}

impl From<String> for DialectKind {
    fn from(s: String) -> Self {
        DialectKind::from(s.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct DialectSpec {
    pub name: &'static str,
    pub keywords: &'static phf::Map<&'static str, Keyword>,
    pub operators: &'static phf::Map<&'static str, Operator>,
    pub style_rules: StyleRules,
    pub follow_rules: &'static [Rules],
}

impl DialectSpec {
    pub fn match_keyword(&self, keyword: &str) -> Option<Keyword> {
        let kw_lookup = match self.style_rules.keywords_case_insensitive {
            true => std::borrow::Cow::Owned(keyword.to_ascii_uppercase()),
            false => std::borrow::Cow::Borrowed(keyword),
        };
        self.keywords.get(&kw_lookup).copied()
    }

    pub fn match_operator(&self, operator: &str) -> Option<Operator> {
        let op_lookup = match self.style_rules.word_ops_case_insensitive {
            true => std::borrow::Cow::Owned(operator.to_ascii_uppercase()),
            false => std::borrow::Cow::Borrowed(operator),
        };
        self.operators.get(&op_lookup).copied()
    }

    pub fn supports_quote_style(&self, quote: QuoteStyle) -> bool {
        self.style_rules.quotes.contains(&quote)
    }

    pub fn supports_comment_style(&self, comment: CommentStyle) -> bool {
        self.style_rules.comments.contains(&comment)
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
        match self.style_rules.unquoted_identifier_fold {
            CaseFold::Upper => std::borrow::Cow::Owned(ident.to_ascii_uppercase()),
            CaseFold::Lower => std::borrow::Cow::Owned(ident.to_ascii_lowercase()),
            CaseFold::Preserve => std::borrow::Cow::Borrowed(ident),
        }
    }

    pub fn resolve_follow_rules(&self, tokens: &[TokenKind]) -> impl Iterator<Item = Vec<Keyword>> {
        resolve_next(self.follow_rules, tokens)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseFold {
    Upper,
    Lower,
    Preserve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleRules {
    /// Are keywords case-insensitive? (ANSI & all major engines: yes)
    pub keywords_case_insensitive: bool,
    /// Are word-operators (AND/OR/NOT/LIKE/ILIKE/IN, etc.) case-insensitive?
    pub word_ops_case_insensitive: bool,
    /// How to fold *unquoted* identifiers
    pub unquoted_identifier_fold: CaseFold,
    /// Quoted identifiers are treated as case-sensitive (ANSI default).
    pub quoted_identifiers_case_sensitive: bool,
    /// What comment styles are supported?
    pub comments: &'static [CommentStyle],
    /// What quote styles are supported?
    pub quotes: &'static [QuoteStyle],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentStyle {
    DoubleDash,
    SlashStar,
    Hash,
}
