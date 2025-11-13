use crate::complete::Completer;
use crate::complete::candidate::Candidate;
use crate::complete::candidate::CandidateSet;
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

pub trait Ranker {
    type State<'ctx>;
    fn init_state<'ctx>(&mut self, ctx: &Context<'ctx>) -> Self::State<'ctx>;
    fn score<'ctx>(
        &self, cand: &Candidate<'ctx>, state: &mut Self::State<'ctx>, ctx: &Context<'ctx>,
    ) -> f32;
}

impl<T: Ranker + std::fmt::Debug> Completer for T {
    fn complete<'ctx>(&mut self, ctx: &mut Context<'ctx>, builder: &mut CandidateSet<'ctx>) {
        // Create per-call state that can borrow from `ctx`
        let mut state = self.init_state(ctx);

        // Score each candidate using the same state
        for cand in builder.items.iter_mut() {
            let score = self.score(cand, &mut state, ctx);
            cand.score = Score(score);
        }
    }
}

#[derive(Debug)]
pub struct DefaultRanker(CompositeRanker<any::AnyRanker>);
impl Default for DefaultRanker {
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

impl Ranker for DefaultRanker {
    type State<'ctx> = <CompositeRanker<any::AnyRanker> as Ranker>::State<'ctx>;
    fn init_state<'ctx>(&mut self, ctx: &Context<'ctx>) -> Self::State<'ctx> {
        self.0.init_state(ctx)
    }
    fn score<'ctx>(
        &self, cand: &Candidate<'ctx>, state: &mut Self::State<'ctx>, ctx: &Context<'ctx>,
    ) -> f32 {
        self.0.score(cand, state, ctx)
    }
}
