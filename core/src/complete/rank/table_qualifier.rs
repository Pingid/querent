use crate::complete::candidate::Candidate;
use crate::complete::candidate::CandidateKind;
use crate::complete::context::Context;
use crate::complete::rank::Ranker;

/// Ranks table completions based on qualification level.
/// Prioritizes unqualified names (e.g., "users") over qualified names (e.g., "public.users")
/// when there's no ambiguity.
#[derive(Debug, Default)]
pub struct TableQualifiedRankState {
    prioritize_unqualified: bool,
}

#[derive(Debug, Default)]
pub struct TableQualifiedRank;
impl Ranker for TableQualifiedRank {
    type State<'ctx> = TableQualifiedRankState;
    fn init_state<'ctx>(&mut self, _ctx: &Context<'ctx>) -> Self::State<'ctx> {
        TableQualifiedRankState {
            prioritize_unqualified: true,
        }
    }
    fn score<'ctx>(
        &self, cand: &Candidate<'ctx>, state: &mut Self::State<'ctx>, _ctx: &Context<'ctx>,
    ) -> f32 {
        match &cand.kind {
            CandidateKind::Table(table) => {
                if state.prioritize_unqualified {
                    // Prioritize unqualified names
                    if table.label.schema.is_none() {
                        return 2.0; // Unqualified (e.g., users)
                    } else {
                        return 1.0; // Schema-qualified (e.g., public.users)
                    }
                }
                1.0
            }
            _ => 0.0,
        }
    }
}
