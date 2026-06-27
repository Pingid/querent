use crate::complete::Completer;
use crate::complete::candidate::CandidateSet;
use crate::complete::context::Context;
use crate::complete::provider::DefaultProviders;
use crate::complete::rank::DefaultRanker;
use crate::complete::types::Completions;
use crate::dialect::DialectSpec;
use crate::doc::Content;
use crate::schema;

pub struct Engine<P, R> {
    pub spec: &'static DialectSpec,
    pub schema: schema::Cache,
    pub providers: P,
    pub ranker: R,
}

impl Engine<DefaultProviders, DefaultRanker> {
    pub fn new(spec: &'static DialectSpec, schema: schema::Cache) -> Self {
        Self {
            spec,
            schema,
            ranker: DefaultRanker::default(),
            providers: DefaultProviders::default(),
        }
    }

    pub fn complete(&mut self, doc: &Content) -> Completions {
        let text = doc.to_string();
        let cursor = doc.cursor().min(text.len());
        let mut candidates = CandidateSet::new();
        let Some(mut ctx) = Context::build(self.spec, &self.schema, &text, cursor) else {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to build context");
            return candidates.empty();
        };
        candidates.replace(ctx.cursor().replace);
        self.providers.complete(&mut ctx, &mut candidates);
        self.ranker.complete(&mut ctx, &mut candidates);
        let completions = candidates.completions();
        completions
    }
}
