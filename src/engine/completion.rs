use crate::{catalog::schema, span::Span};

#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub label: String,
    pub insert_text: String,
    pub filter_text: Option<String>,
    pub kind: CompletionKind,
    pub replace: Span,
    pub commit_characters: Vec<char>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Table(TableCompletion), // Option<String> schema
    Column(ColumnCompletion),
    Function,
    Operator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnCompletion {
    pub qualifier: Option<String>, // schema.table
    pub column: Option<schema::Column>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCompletion {
    pub qualifier: Option<String>, // schema
    pub table: Option<schema::Table>,
}
