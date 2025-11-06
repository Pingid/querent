use crate::complete::context::QualifiedIdent;
use crate::complete::types::Completion;
use crate::complete::types::CompletionKind;
use crate::complete::types::Completions;
use crate::complete::types::InsertTextFormat;
use crate::schema;
use crate::span::Span;

#[derive(Debug, Default)]
pub struct CandidateSet<'a> {
    pub items: Vec<Candidate<'a>>,
}

impl<'a> CandidateSet<'a> {
    pub fn push(&mut self, candidate: Candidate<'a>) {
        self.items.push(candidate);
    }
    pub fn completions(mut self) -> Completions {
        self.items.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.completion.label.cmp(&b.completion.label))
        });
        Completions {
            items: self.items.into_iter().map(|c| c.completion).collect(),
        }
    }
    pub fn empty(self) -> Completions {
        Completions { items: vec![] }
    }
}

#[derive(Debug, Clone)]
pub struct Candidate<'a> {
    pub completion: Completion,
    pub kind: CandidateKind<'a>,
    pub score: f32,
}

impl<'a> Candidate<'a> {
    pub fn new(kind: CandidateKind<'a>) -> Self {
        let completion = Completion {
            label: "".to_string(),
            kind: kind.completion_kind(),
            detail: None,
            insert_text: "".to_string(),
            filter_text: None,
            replace: Span::new(0, 0),
            commit_characters: Vec::new(),
            insert_text_format: InsertTextFormat::PlainText,
        };
        Self {
            completion,
            score: 0.0,
            kind,
        }
    }

    pub fn column(
        label: QualifiedIdent<'a>, ident: QualifiedIdent<'a>, dt: Option<schema::DataType>,
    ) -> Self {
        Self::new(CandidateKind::Column(ColumnCandidate { label, ident, dt }))
            .label(label.to_string())
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.completion.label = label.into();
        if self.completion.insert_text.is_empty() {
            self.completion.insert_text = self.completion.label.clone();
        }
        self
    }

    pub fn score(mut self, score: f32) -> Self {
        self.score = score;
        self
    }

    // Completion attributes
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.completion.detail = Some(detail.into());
        self
    }
    pub fn insert_text(mut self, insert_text: impl Into<String>) -> Self {
        self.completion.insert_text = insert_text.into();
        self
    }
    pub fn filter_text(mut self, filter_text: impl Into<String>) -> Self {
        self.completion.filter_text = Some(filter_text.into());
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

#[derive(Debug, Clone, Copy)]
pub enum CandidateKind<'a> {
    Column(ColumnCandidate<'a>),
    Function(FunctionCandidate<'a>),
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

#[derive(Debug, Clone, Copy)]
pub struct ColumnCandidate<'a> {
    /// The completion label
    pub label: QualifiedIdent<'a>,
    /// The identifier of the column
    pub ident: QualifiedIdent<'a>,
    /// The data type of the column
    pub dt: Option<schema::DataType>,
}

#[derive(Debug, Clone, Copy)]
pub struct FunctionCandidate<'a> {
    pub schema: Option<&'a schema::Function>,
    pub return_type: Option<schema::DataType>,
}
