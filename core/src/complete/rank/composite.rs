use crate::complete::candidate::Candidate;
#[cfg(any(test, feature = "test-utils"))]
use crate::complete::candidate::CandidateLineage;
use crate::complete::context::Context;
use crate::complete::rank::Ranker;

/// A ranker that aggregates multiple rankers (weighted linear combo).
pub struct CompositeRankerState<'ctx, R: Ranker> {
    parts: Vec<(R::State<'ctx>, f32)>,
}

#[derive(Debug, Default)]
pub struct CompositeRanker<R> {
    parts: Vec<(R, f32)>,
}

impl<R: Ranker> CompositeRanker<R> {
    pub fn new() -> Self {
        Self { parts: vec![] }
    }

    pub fn with(mut self, r: impl Into<R>, weight: f32) -> Self {
        self.parts.push((r.into(), weight));
        self
    }
}

impl<R: Ranker + std::fmt::Debug> Ranker for CompositeRanker<R> {
    type State<'ctx> = CompositeRankerState<'ctx, R>;

    fn init_state<'ctx>(&mut self, ctx: &Context<'ctx>) -> Self::State<'ctx> {
        CompositeRankerState {
            parts: self
                .parts
                .iter_mut()
                .map(|(r, w)| (r.init_state(ctx), *w))
                .collect(),
        }
    }

    fn score<'ctx>(
        &self, cand: &Candidate<'ctx>, state: &mut Self::State<'ctx>, ctx: &Context<'ctx>,
    ) -> f32 {
        self.parts
            .iter()
            .zip(state.parts.iter_mut())
            .map(|((r, w), (sub_state, _w_state))| {
                let score = r.score(cand, sub_state, ctx);

                #[cfg(any(test, feature = "test-utils"))]
                cand.add_lineage(CandidateLineage::Ranked(format!("{:?}", r), score * *w));

                *w * score
            })
            .sum()
    }
}
