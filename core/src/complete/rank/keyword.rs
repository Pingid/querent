use crate::complete::candidate::Candidate;
use crate::complete::candidate::CandidateKind;
use crate::complete::context::Context;
use crate::complete::rank::Ranker;
use crate::lex::Keyword;

#[derive(Debug, Default)]
pub struct KeywordMatchRank;
impl Ranker for KeywordMatchRank {
    type State<'ctx> = ();
    fn init_state<'ctx>(&mut self, _ctx: &Context<'ctx>) -> Self::State<'ctx> {
    }
    fn score<'ctx>(
        &self, cand: &Candidate<'ctx>, _state: &mut Self::State<'ctx>, _ctx: &Context<'ctx>,
    ) -> f32 {
        let CandidateKind::Keyword(Some(keyword)) = cand.kind else {
            return 0.0;
        };
        match keyword {
            // Core query keywords - highest priority
            Keyword::Select | Keyword::From => 1.0,
            // Common clauses - very high priority
            Keyword::Where | Keyword::Join => 0.9,
            // CTEs and grouping - high priority
            Keyword::With | Keyword::Group | Keyword::Having => 0.85,
            // DML operations - high priority
            Keyword::Insert | Keyword::Update | Keyword::Delete => 0.8,
            // DDL operations - medium-high priority
            Keyword::Create | Keyword::Alter | Keyword::Drop => 0.75,
            // Ordering and limiting - medium priority
            Keyword::Order | Keyword::Limit | Keyword::Offset => 0.7,
            // Set operations - medium priority
            Keyword::Union | Keyword::Intersect | Keyword::Except => 0.65,
            // Window functions - medium priority
            Keyword::Over | Keyword::Partition => 0.6,
            // Advanced operations - lower priority
            Keyword::Merge => 0.5,
            // Default for unmatched keywords
            _ => 0.4,
        }
    }
}
