use crate::{dialect::DialectSpec, doc::Content, schema};

mod completion;
mod context;
mod provider;

pub use completion::*;
pub use context::Context;

pub struct Engine {
    pub spec: &'static DialectSpec,
    pub schema: schema::Cache,
}

impl Engine {
    pub fn new(spec: &'static DialectSpec, schema: schema::Cache) -> Self {
        Self { spec, schema }
    }

    pub fn complete(&self, doc: &Content) -> Vec<Completion> {
        complete(&self.spec, &self.schema, doc)
    }
}

pub fn complete(spec: &DialectSpec, schema: &schema::Cache, doc: &Content) -> Vec<Completion> {
    let text = doc.to_string();
    let cursor = doc.cursor().min(text.len());
    let Some(ctx) = Context::build(spec, schema, &text, cursor) else {
        return vec![];
    };
    let mut builder = CompletionBuilder::new();
    provider::complete(&ctx, &mut builder);
    builder.build(&ctx)
}
