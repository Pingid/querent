use crate::complete::Completer;
use crate::complete::candidate::CandidateSet;
use crate::complete::context::Context;
use crate::complete::provider::DefaultProviders;
use crate::complete::rank::DefaultRanker;
use crate::complete::types::Completions;
use crate::dialect::DialectSpec;
use crate::doc::Content;
use crate::schema;

pub struct Engine {
    pub spec: &'static DialectSpec,
    pub schema: schema::Cache,
}

impl Engine {
    pub fn new(spec: &'static DialectSpec, schema: schema::Cache) -> Self {
        Self { spec, schema }
    }

    pub fn complete(&self, doc: &Content) -> Completions {
        complete(&self.spec, &self.schema, doc)
    }
}

pub fn complete(spec: &DialectSpec, schema: &schema::Cache, doc: &Content) -> Completions {
    let text = doc.to_string();
    let cursor = doc.cursor().min(text.len());
    let mut candidates = CandidateSet::default();
    let Some(mut ctx) = Context::build(spec, schema, &text, cursor) else {
        return candidates.empty();
    };
    DefaultProviders.complete(&mut ctx, &mut candidates);
    DefaultRanker::default().complete(&mut ctx, &mut candidates);
    candidates.completions()
}
