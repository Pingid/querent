use crate::complete::Completer;
use crate::complete::completion::Candidate;
use crate::complete::completion::CandidateKind;
use crate::complete::completion::ColumnCandidate;
use crate::complete::context::Context;

mod any;
mod basic;
mod column_qualifier;
mod column_source;
mod composite;

pub use any::AnyRanker;

/// Rankers only *score*. Keep them composable.
pub trait Ranker<'a> {
    fn prepare(&mut self, _ctx: &Context<'a>) {}
    fn score(&self, ctx: &Context<'a>, cand: &Candidate<'a>) -> f32;
}

impl<'a, T: Ranker<'a>> Completer<'a> for T {
    fn complete(
        &mut self, ctx: &mut Context<'a>, builder: &mut super::completion::CandidateSet<'a>,
    ) {
        self.prepare(ctx);
        for cand in builder.items.iter_mut() {
            cand.score = self.score(ctx, cand);
        }
    }
}

/// Rankers that only score columns.
pub trait ColumnRanker<'a> {
    fn prepare(&mut self, _ctx: &Context<'a>) {}
    fn score_column(
        &self, ctx: &Context<'a>, cand: &Candidate<'a>, col: &ColumnCandidate<'a>,
    ) -> f32;
}

impl<'a, R: ColumnRanker<'a>> Ranker<'a> for R {
    fn prepare(&mut self, ctx: &Context<'a>) {
        self.prepare(ctx);
    }
    fn score(&self, ctx: &Context<'a>, cand: &Candidate<'a>) -> f32 {
        match cand.kind {
            CandidateKind::Column(col) => self.score_column(ctx, cand, &col),
            _ => 0.0,
        }
    }
}
