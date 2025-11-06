use crate::complete::candidate::Candidate;
use crate::complete::candidate::CandidateKind;
use crate::complete::context::Context;
use crate::complete::rank::Ranker;

/// Ranks table completions based on qualification level.
/// Prioritizes unqualified names (e.g., "users") over qualified names (e.g., "public.users")
/// when there's no ambiguity.
pub struct TableQualifiedRank {
    prioritize_unqualified: bool,
}

impl Default for TableQualifiedRank {
    fn default() -> Self {
        Self {
            prioritize_unqualified: true,
        }
    }
}

impl<'a> Ranker<'a> for TableQualifiedRank {
    fn prepare(&mut self, _ctx: &Context<'a>) {
        // For now, always prioritize unqualified names
        // In the future, we could check if there are schema qualifiers in the query
        self.prioritize_unqualified = true;
    }

    fn score(&self, _: &Context<'_>, cand: &Candidate) -> f32 {
        match &cand.kind {
            CandidateKind::Table(table) => {
                if self.prioritize_unqualified {
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
