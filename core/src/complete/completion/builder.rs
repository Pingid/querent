use super::super::context::Context;
use super::Completion;
use super::ranker::DefaultRanker;
use super::ranker::Ranker;
use crate::complete::completion::Completions;
use crate::schema;

#[derive(Debug, Clone, PartialEq)]
pub struct CompletionBuilder {
    pub items: Vec<CompletionWithScore>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompletionWithScore {
    pub completion: Completion,
    pub data_type: Option<schema::DataType>,
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

    pub fn add(&mut self, item: Completion, score: i8, data_type: Option<schema::DataType>) {
        self.items.push(CompletionWithScore {
            completion: item,
            score,
            data_type,
        });
    }

    pub fn items(&mut self) -> &mut Vec<CompletionWithScore> {
        &mut self.items
    }

    pub fn build(self, ctx: &Context) -> Completions {
        let ranked = DefaultRanker::default().rank(&ctx.cursor.fragment, self.items);
        Completions {
            items: ranked
                .into_iter()
                // .take(100)
                .map(|CompletionWithScore { completion, .. }| Completion {
                    label: completion.label,
                    insert_text: completion.insert_text,
                    filter_text: completion.filter_text,
                    kind: completion.kind,
                    replace: ctx.cursor.replace,
                    commit_characters: completion.commit_characters,
                    insert_text_format: completion.insert_text_format,
                    detail: completion.detail,
                })
                .collect(),
        }
    }

    pub fn empty(self) -> Completions {
        Completions { items: vec![] }
    }
}
