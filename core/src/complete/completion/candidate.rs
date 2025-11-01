use crate::complete::completion::Completion;
use crate::complete::completion::CompletionKind;
use crate::complete::completion::InsertTextFormat;
use crate::schema;
use crate::span::Span;

#[derive(Debug, Default)]
pub struct CandidateSet<'a> {
    pub items: Vec<Candidate<'a>>,
}

impl<'a> CandidateSet<'a> {
    pub fn with(mut self, candidate: Candidate<'a>) -> Self {
        self.items.push(candidate);
        self
    }
}

#[derive(Debug)]
pub struct Candidate<'a> {
    pub completion: Completion,
    pub kind: CandidateKind<'a>,
    pub score: usize,
}

impl<'a> Candidate<'a> {
    pub fn new(label: String, kind: CandidateKind<'a>) -> Self {
        let completion = Completion {
            label: label.clone(),
            kind: kind.completion_kind(),
            detail: None,
            insert_text: label.clone(),
            filter_text: None,
            replace: Span::new(0, 0),
            commit_characters: Vec::new(),
            insert_text_format: InsertTextFormat::PlainText,
        };
        Self {
            completion,
            score: 0,
            kind,
        }
    }

    pub fn score(mut self, score: usize) -> Self {
        self.score = score;
        self
    }

    // Completion attributes
    pub fn detail(mut self, detail: String) -> Self {
        self.completion.detail = Some(detail);
        self
    }
    pub fn insert_text(mut self, insert_text: String) -> Self {
        self.completion.insert_text = insert_text;
        self
    }
    pub fn filter_text(mut self, filter_text: String) -> Self {
        self.completion.filter_text = Some(filter_text);
        self
    }
    pub fn replace(mut self, replace: Span) -> Self {
        self.completion.replace = replace;
        self
    }
    pub fn commit_characters(mut self, commit_characters: Vec<char>) -> Self {
        self.completion.commit_characters = commit_characters;
        self
    }
    pub fn insert_text_format(mut self, insert_text_format: InsertTextFormat) -> Self {
        self.completion.insert_text_format = insert_text_format;
        self
    }
}

#[derive(Debug)]
pub enum CandidateKind<'a> {
    Column(ColumnCandidate<'a>),
    Function {
        name: &'a str,
        dt: Option<schema::DataType>,
    },
    Keyword,
    Table,
    Operator,
}

impl<'a> CandidateKind<'a> {
    pub fn completion_kind(&self) -> CompletionKind {
        match self {
            CandidateKind::Column(_) => CompletionKind::Column,
            CandidateKind::Function { .. } => CompletionKind::Function,
            CandidateKind::Keyword => CompletionKind::Keyword,
            CandidateKind::Table => CompletionKind::Table,
            CandidateKind::Operator => CompletionKind::Operator,
        }
    }
}

#[derive(Debug)]
pub struct ColumnCandidate<'a> {
    pub dt: Option<schema::DataType>,
    pub col: Option<&'a schema::Column>,
    pub name: &'a str,
    pub scope_alias: Option<&'a str>,
}

// Schema {
//     column: &'a schema::Column,
// },
// Cte {
//     cte: &'a str,
//     dt: Option<schema::DataType>,
//     name: &'a str,
// },
// Unresolved {
//     dt: Option<schema::DataType>,
//     name: &'a str,
// },
// Literal {
//     dt: schema::DataType,
//     name: &'a str,
// },

#[derive(Debug)]
pub struct FunctionCandidate<'a> {
    schema: Option<&'a schema::Function>,
    return_type: Option<schema::DataType>,
}
