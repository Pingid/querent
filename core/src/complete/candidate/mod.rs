use std::collections::HashSet;

use smol_str::SmolStr;

use crate::complete::context::QualifiedIdent;
use crate::complete::types::Completion;
use crate::complete::types::CompletionKind;
use crate::complete::types::Completions;
use crate::complete::types::InsertTextFormat;
use crate::lex::Keyword;
use crate::schema;
use crate::span::Span;

#[derive(Debug, Default)]
pub struct CandidateSet<'a> {
    pub items: Vec<Candidate<'a>>,
    pub seen: HashSet<SmolStr>,
}

impl<'a> CandidateSet<'a> {
    pub fn push(&mut self, candidate: Candidate<'a>) {
        if self.seen.insert(candidate.completion.insert_text.clone()) {
            self.items.push(candidate);
        }
    }
    pub fn completions(mut self) -> Completions {
        self.items
            .sort_by_key(|c| (std::cmp::Reverse(c.score), c.completion.label.clone()));
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
    pub score: Score,
    #[cfg(any(test, feature = "test-utils"))]
    pub lineage: std::cell::RefCell<Vec<CandidateLineage>>,
}

impl<'a> Candidate<'a> {
    #[cfg(any(test, feature = "test-utils"))]
    pub fn add_lineage(&self, lineage: CandidateLineage) {
        self.lineage.borrow_mut().push(lineage);
    }
}

#[derive(Debug, Clone)]
pub struct CandidateBuilder<'a> {
    kind: CandidateKind<'a>,
    label: SmolStr,
    detail: Option<SmolStr>,
    insert_text: Option<SmolStr>,
    filter_text: Option<SmolStr>,
    replace: Span,
    commit_characters: Option<Vec<char>>,
    insert_text_format: InsertTextFormat,
    score: Score,
}

impl<'a> CandidateBuilder<'a> {
    pub fn new(kind: CandidateKind<'a>, label: impl Into<SmolStr>) -> Self {
        Self {
            kind,
            label: label.into(),
            detail: None,
            insert_text: None,
            filter_text: None,
            replace: Span::new(0, 0),
            commit_characters: None, // engine/client defaults if None
            insert_text_format: InsertTextFormat::PlainText,
            score: Score(0.0),
        }
    }

    pub fn column(
        label: QualifiedIdent<'a>, ident: QualifiedIdent<'a>, dt: Option<schema::DataType>,
    ) -> Self {
        Self::new(
            CandidateKind::Column(ColumnCandidate { label, ident, dt }),
            label.to_string(),
        )
    }

    pub fn function(
        name: &str, return_type: Option<schema::DataType>, parameter_types: &'a [schema::DataType],
    ) -> Self {
        Self::new(
            CandidateKind::Function(FunctionCandidate {
                return_type,
                parameter_types,
            }),
            name,
        )
    }

    pub fn keyword(label: impl Into<SmolStr>, keyword: Option<Keyword>) -> Self {
        Self::new(CandidateKind::Keyword(keyword), label.into())
    }

    pub fn table(label: QualifiedIdent<'a>, ident: QualifiedIdent<'a>) -> Self {
        Self::new(
            CandidateKind::Table(TableCandidate { label, ident }),
            label.to_string(),
        )
    }

    pub fn detail(mut self, d: impl Into<SmolStr>) -> Self {
        self.detail = Some(d.into());
        self
    }
    pub fn insert_text(mut self, t: impl Into<SmolStr>) -> Self {
        self.insert_text = Some(t.into());
        self
    }
    pub fn filter_text(mut self, t: impl Into<SmolStr>) -> Self {
        self.filter_text = Some(t.into());
        self
    }
    pub fn replace(mut self, span: Span) -> Self {
        self.replace = span;
        self
    }
    pub fn commit_chars(mut self, chars: Vec<char>) -> Self {
        self.commit_characters = Some(chars);
        self
    }
    pub fn snippet(mut self) -> Self {
        self.insert_text_format = InsertTextFormat::Snippet;
        self
    }
    pub fn score(mut self, s: f32) -> Self {
        self.score = Score(s);
        self
    }

    pub fn build(self) -> Candidate<'a> {
        assert!(!self.label.is_empty(), "label must not be empty");

        let mut completion = Completion::new(
            self.kind.into(),
            self.label.clone(),
            self.replace,
            self.commit_characters.clone(),
            self.detail.clone(),
        );

        if let Some(t) = self.insert_text {
            completion.insert_text = t;
        }
        if let Some(f) = self.filter_text {
            completion.filter_text = Some(f);
        }
        completion.insert_text_format = self.insert_text_format;

        Candidate {
            completion,
            kind: self.kind,
            score: self.score.into(),
            #[cfg(any(test, feature = "test-utils"))]
            lineage: std::cell::RefCell::new(vec![]),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CandidateKind<'a> {
    Column(ColumnCandidate<'a>),
    Function(FunctionCandidate<'a>),
    Keyword(Option<Keyword>),
    Table(TableCandidate<'a>),
    Operator,
}

impl<'a> Into<CompletionKind> for CandidateKind<'a> {
    fn into(self) -> CompletionKind {
        match self {
            CandidateKind::Column(_) => CompletionKind::Column,
            CandidateKind::Function { .. } => CompletionKind::Function,
            CandidateKind::Keyword(_) => CompletionKind::Keyword,
            CandidateKind::Table(_) => CompletionKind::Table,
            CandidateKind::Operator => CompletionKind::Operator,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ColumnCandidate<'a> {
    /// The completion label as parts
    pub label: QualifiedIdent<'a>,
    /// The full identifier of the column
    pub ident: QualifiedIdent<'a>,
    /// The data type of the column
    pub dt: Option<schema::DataType>,
}

#[derive(Debug, Clone, Copy)]
pub struct TableCandidate<'a> {
    /// The completion label as parts
    pub label: QualifiedIdent<'a>,
    /// The full identifier of the table
    pub ident: QualifiedIdent<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct FunctionCandidate<'a> {
    pub parameter_types: &'a [schema::DataType],
    pub return_type: Option<schema::DataType>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Score(pub f32);
impl Eq for Score {}
impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Score {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, Clone)]
pub enum CandidateLineage {
    Provider(String),
    Ranked(String, f32),
}

#[cfg(any(test, feature = "test-utils"))]
impl std::fmt::Display for CandidateLineage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CandidateLineage::Provider(name) => write!(f, "Provider: {}", name),
            CandidateLineage::Ranked(name, score) => {
                write!(f, "{} ({})", name, score)
            }
        }
    }
}
