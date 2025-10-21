use crate::complete::Completions;

use super::super::context::Context;
use super::Completion;
use super::ranker::{DefaultRanker, Ranker};

#[derive(Debug, Clone, PartialEq)]
pub struct CompletionBuilder {
    pub items: Vec<CompletionWithScore>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompletionWithScore {
    pub completion: Completion,
    pub score: i8,
}

impl std::ops::Deref for CompletionWithScore {
    type Target = Completion;
    fn deref(&self) -> &Self::Target {
        &self.completion
    }
}

impl CompletionBuilder {
    pub fn new() -> Self {
        Self { items: vec![] }
    }

    pub fn add(&mut self, item: Completion, score: i8) {
        self.items.push(CompletionWithScore {
            completion: item,
            score,
        });
    }

    pub fn build(self, ctx: &Context) -> Completions {
        let ranked = DefaultRanker::default().rank(&ctx.cursor.fragment, self.items);
        Completions {
            items: ranked
                .into_iter()
                .map(|CompletionWithScore { completion, .. }| Completion {
                    label: completion.label,
                    insert_text: completion.insert_text,
                    filter_text: completion.filter_text,
                    kind: completion.kind,
                    replace: ctx.cursor.replace,
                    commit_characters: completion.commit_characters,
                    detail: completion.detail,
                })
                .collect(),
        }
    }

    pub fn empty(self) -> Completions {
        Completions { items: vec![] }
    }
}
