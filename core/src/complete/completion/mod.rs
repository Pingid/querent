use crate::{schema, span::Span};

mod builder;
mod ranker;

pub use builder::*;

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
pub struct Completions {
    pub items: Vec<Completion>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub label: String,
    pub detail: Option<String>,
    pub insert_text: String,
    pub filter_text: Option<String>,
    pub kind: CompletionKind,
    pub replace: Span,
    pub commit_characters: Vec<char>,
}

impl Completion {
    pub fn new(
        kind: CompletionKind,
        label: String,
        replace: Span,
        commit_characters: Option<Vec<char>>,
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
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Table, // Option<String> schema
    Column,
    Schema,
    Function,
    Operator,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnCompletion {
    pub qualifier: Option<String>, // schema.table
    pub column: Option<schema::Column>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCompletion {
    pub qualifier: Option<String>, // schema
    pub table: Option<schema::Table>,
}
