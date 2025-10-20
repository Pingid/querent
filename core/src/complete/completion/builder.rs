use crate::complete::Completions;

use super::super::context::Context;
use super::ranker::{DefaultRanker, DefaultScorer, Ranker};
use super::{Completion, CompletionKind};

#[derive(Debug, Clone, PartialEq)]
pub struct CompletionBuilder {
    pub items: Vec<PossibleCompletion>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PossibleCompletion {
    pub label: String,
    pub insert_text: String,
    pub filter_text: Option<String>,
    pub kind: CompletionKind,
    pub commit_characters: Vec<char>,
    pub score: i8,
}

impl CompletionBuilder {
    pub fn new() -> Self {
        Self { items: vec![] }
    }

    pub fn add(&mut self, item: PossibleCompletion) {
        self.items.push(item);
    }

    pub fn build(self, ctx: &Context) -> Completions {
        let ranked = DefaultRanker::new(DefaultScorer).rank(&ctx.cursor.fragment, self.items);
        Completions {
            items: ranked
                .into_iter()
                .map(|item| Completion {
                    label: item.label,
                    insert_text: item.insert_text,
                    filter_text: item.filter_text,
                    kind: item.kind,
                    replace: ctx.cursor.replace,
                    commit_characters: item.commit_characters,
                })
                .collect(),
        }
    }

    pub fn empty(self) -> Completions {
        Completions { items: vec![] }
    }
}
