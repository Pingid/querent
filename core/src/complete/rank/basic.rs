use crate::complete::completion::Candidate;
use crate::complete::completion::CandidateKind;
use crate::complete::context::Context;
use crate::complete::rank::Ranker;

pub struct KindMatchRank;
impl<'a> Ranker<'a> for KindMatchRank {
    fn score(&self, _: &Context<'_>, cand: &Candidate) -> f32 {
        match cand.kind {
            CandidateKind::Column(_) => 1.0,
            CandidateKind::Table => 0.9,
            CandidateKind::Function(_) => 0.9,
            CandidateKind::Keyword => 0.8,
            CandidateKind::Operator => 0.7,
        }
    }
}

pub struct TypeCompatRank;
impl<'a> Ranker<'a> for TypeCompatRank {
    fn score(&self, ctx: &Context<'_>, cand: &Candidate) -> f32 {
        if let Some(expected) = ctx.expected_data_type() {
            match &cand.kind {
                CandidateKind::Column(col) => {
                    if col.dt == Some(expected) {
                        return 1.0;
                    }
                }
                CandidateKind::Function(func) => {
                    if func.return_type == Some(expected) {
                        return 1.0;
                    }
                }
                _ => {}
            }
        }
        0.0
    }
}
