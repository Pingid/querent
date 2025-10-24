use crate::span::Span;

mod builder;
mod ranker;

pub use builder::*;

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Completions {
    pub items: Vec<Completion>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub label: String,
    pub detail: Option<String>,
    pub insert_text: String,
    pub filter_text: Option<String>,
    pub kind: CompletionKind,
    pub replace: Span,
    pub commit_characters: Vec<char>,
    pub insert_text_format: InsertTextFormat,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum InsertTextFormat {
    #[default]
    PlainText,
    Snippet,
}

impl Completion {
    pub fn new(
        kind: CompletionKind, label: String, replace: Span, commit_characters: Option<Vec<char>>,
        detail: Option<String>,
    ) -> Self {
        Self {
            label: label.clone(),
            detail,
            insert_text: label.clone(),
            filter_text: Some(label.clone()),
            kind,
            replace,
            commit_characters: commit_characters.unwrap_or(vec![',', ' ', '\n']),
            insert_text_format: InsertTextFormat::default(),
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Table, // Option<String> schema
    Column,
    Schema,
    Function,
    Operator,
}
