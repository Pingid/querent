use crate::complete::completion::CandidateSet;
use crate::complete::context::Context;
use crate::dialect::DialectSpec;
use crate::doc::Content;
use crate::schema;

pub mod completion;
pub mod context;
pub mod provider;
pub mod providers;
pub mod rank;

#[cfg(test)]
pub mod test_util;

pub trait Completer<'a> {
    fn complete(&mut self, ctx: &mut Context<'a>, builder: &mut CandidateSet<'a>);
    fn should_complete(&self, _ctx: &Context<'a>) -> bool {
        true
    }
}

pub struct Engine {
    pub spec: &'static DialectSpec,
    pub schema: schema::Cache,
}

impl Engine {
    pub fn new(spec: &'static DialectSpec, schema: schema::Cache) -> Self {
        Self { spec, schema }
    }

    pub fn complete(&self, doc: &Content) -> completion::Completions {
        complete(&self.spec, &self.schema, doc)
    }
}

pub fn complete(
    spec: &DialectSpec, schema: &schema::Cache, doc: &Content,
) -> completion::Completions {
    let text = doc.to_string();
    let cursor = doc.cursor().min(text.len());
    let mut builder = completion::CompletionBuilder::new();
    let Some(mut ctx) = context::Context::build(spec, schema, &text, cursor) else {
        return builder.empty();
    };
    // provider::complete(&mut ctx, &mut builder);
    builder.build(&ctx)
}
