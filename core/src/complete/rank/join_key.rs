use crate::complete::candidate::Candidate;
use crate::complete::candidate::CandidateKind;
use crate::complete::context::Context;
use crate::complete::context::QualifiedIdent;
use crate::complete::rank::Ranker;

/// Boost columns whose name matches the opposite operand of a comparison by a
/// foreign-key naming heuristic, so `ON c.user_id = u.^` ranks `id` first.
#[derive(Debug, Default)]
pub struct JoinKeyMatchRank;

#[derive(Debug, Default)]
pub struct JoinKeyMatchRankState<'a> {
    operand: Option<QualifiedIdent<'a>>,
}

impl Ranker for JoinKeyMatchRank {
    type State<'ctx> = JoinKeyMatchRankState<'ctx>;
    fn init_state<'ctx>(&mut self, ctx: &Context<'ctx>) -> Self::State<'ctx> {
        JoinKeyMatchRankState {
            operand: ctx.comparison_operand().copied(),
        }
    }
    fn score<'ctx>(
        &self, cand: &Candidate<'ctx>, state: &mut Self::State<'ctx>, _ctx: &Context<'ctx>,
    ) -> f32 {
        let CandidateKind::Column(col) = &cand.kind else {
            return 0.0;
        };
        let Some(operand) = &state.operand else {
            return 0.0;
        };
        match names_relate(operand.name(), col.ident.name()) {
            true => 1.0,
            false => 0.0,
        }
    }
}

/// Heuristic: do two column names look like a foreign-key pair?
/// Matches identical names or a `<prefix>_<other>` relationship
/// (e.g. `user_id` ~ `id`, `post` ~ `post_id`).
fn names_relate(a: &str, b: &str) -> bool {
    a == b || a.ends_with(&format!("_{b}")) || b.ends_with(&format!("_{a}"))
}
