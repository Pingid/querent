use crate::complete::candidate::Candidate;
#[cfg(any(test, feature = "test-utils"))]
use crate::complete::candidate::CandidateLineage;
use crate::complete::context::Context;
use crate::complete::rank::Ranker;

/// A ranker that aggregates multiple rankers (weighted linear combo).
pub struct CompositeRanker<R> {
    parts: Vec<(R, f32)>,
}

impl<'a, R: Ranker<'a>> CompositeRanker<R> {
    pub fn new() -> Self {
        Self { parts: vec![] }
    }
    pub fn with(mut self, r: impl Into<R>, weight: f32) -> Self {
        self.parts.push((r.into(), weight));
        self
    }
}

impl<'a, R: Ranker<'a> + std::fmt::Debug> Ranker<'a> for CompositeRanker<R> {
    fn prepare(&mut self, ctx: &Context<'a>) {
        for (r, _) in &mut self.parts {
            r.prepare(ctx);
        }
    }
    fn score(&self, ctx: &Context<'a>, cand: &Candidate<'a>) -> f32 {
        self.parts
            .iter()
            .map(|(r, w)| {
                let score = r.score(ctx, cand);

                #[cfg(any(test, feature = "test-utils"))]
                cand.add_lineage(CandidateLineage::Ranked(format!("{:?}", r), score * w));

                w * score
            })
            .sum()
    }
}
