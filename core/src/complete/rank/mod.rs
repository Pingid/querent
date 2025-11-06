use enum_dispatch::enum_dispatch;

use crate::complete::Completer;
use crate::complete::candidate::Candidate;
use crate::complete::candidate::CandidateKind;
use crate::complete::candidate::CandidateSet;
use crate::complete::candidate::ColumnCandidate;
use crate::complete::context::Context;
use crate::complete::rank::composite::CompositeRanker;

mod basic;
mod column_qualifier;
mod column_source;
mod composite;

#[enum_dispatch(AnyRanker)]
pub trait Ranker<'a> {
    fn prepare(&mut self, _ctx: &Context<'a>) {}
    fn score(&self, ctx: &Context<'a>, cand: &Candidate<'a>) -> f32;
}

impl<'a, T: Ranker<'a>> Completer<'a> for T {
    fn complete(&mut self, ctx: &mut Context<'a>, builder: &mut CandidateSet<'a>) {
        self.prepare(ctx);
        for cand in builder.items.iter_mut() {
            cand.score = self.score(ctx, cand);
        }
    }
}

#[enum_dispatch]
pub enum AnyRanker<'a> {
    ColumnSource(column_source::ColumnSourceRank<'a>),
    ColumnQualified(column_qualifier::ColumnQualifiedRank),
    KindMatch(basic::KindMatchRank),
    TypeCompat(basic::TypeCompatRank),
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

pub struct DefaultRanker<'a>(CompositeRanker<AnyRanker<'a>>);
impl<'a> DefaultRanker<'a> {
    pub fn new() -> Self {
        Self(
            CompositeRanker::new()
                .with(basic::KindMatchRank, 1.0)
                .with(basic::TypeCompatRank, 1.0)
                .with(column_qualifier::ColumnQualifiedRank::new(), 1.0)
                .with(column_source::ColumnSourceRank::new(), 1.0),
        )
    }
}

impl<'a> Ranker<'a> for DefaultRanker<'a> {
    fn prepare(&mut self, ctx: &Context<'a>) {
        self.0.prepare(ctx);
    }
    fn score(&self, ctx: &Context<'a>, cand: &Candidate<'a>) -> f32 {
        self.0.score(ctx, cand)
    }
}
