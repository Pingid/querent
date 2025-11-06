use crate::complete::Completer;
use crate::complete::candidate::Candidate;
use crate::complete::candidate::CandidateKind;
use crate::complete::candidate::CandidateSet;
use crate::complete::candidate::ColumnCandidate;
use crate::complete::candidate::TableCandidate;
use crate::complete::candidate::Score;
use crate::complete::context::Context;
use crate::complete::rank::composite::CompositeRanker;

mod any;
mod basic;
mod column_qualifier;
mod column_source;
mod composite;
mod keyword;
mod table_qualifier;

pub trait Ranker<'a> {
    fn prepare(&mut self, _ctx: &Context<'a>) {}
    fn score(&self, ctx: &Context<'a>, cand: &Candidate<'a>) -> f32;

    #[cfg(test)]
    fn debug_scores(&self) -> std::collections::HashMap<String, Vec<(String, f32, f32)>> {
        std::collections::HashMap::new()
    }
}

impl<'a, T: Ranker<'a>> Completer<'a> for T {
    fn complete(&mut self, ctx: &mut Context<'a>, builder: &mut CandidateSet<'a>) {
        self.prepare(ctx);

        for cand in builder.items.iter_mut() {
            cand.score = Score(self.score(ctx, cand));
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

/// Rankers that only score tables.
pub trait TableRanker<'a> {
    fn prepare(&mut self, _ctx: &Context<'a>) {}
    fn score_table(
        &self, ctx: &Context<'a>, cand: &Candidate<'a>, table: &TableCandidate<'a>,
    ) -> f32;
}

pub struct DefaultRanker<'a>(CompositeRanker<any::AnyRanker<'a>>);
impl<'a> Default for DefaultRanker<'a> {
    fn default() -> Self {
        Self(
            CompositeRanker::new()
                // Context and semantic rankers (30% of total weight)
                .with(column_qualifier::ColumnQualifiedRank::default(), 2.0)
                .with(column_source::ColumnSourceRank::default(), 2.5)
                .with(table_qualifier::TableQualifiedRank::default(), 2.0)
                .with(keyword::KeywordMatchRank, 1.5)
                .with(basic::KindMatchRank, 2.0)
                .with(basic::TypeCompatRank, 3.0) // Strong type safety signal
                .with(basic::IgnoreRank, 0.5) // Minor filtering factor
                // String matching rankers (70% of total weight)
                .with(basic::ExactMatchRanker, 5.0) // Strong exact match signal
                .with(basic::PrefixMatchRanker, 3.0) // Good prefix match signal
                .with(basic::FuzzyMatchRanker, 1.5), // Fuzzy as fallback
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
