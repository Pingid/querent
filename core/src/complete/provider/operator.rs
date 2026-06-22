use crate::complete::Completer;
use crate::complete::candidate::CandidateBuilder;
use crate::complete::candidate::CandidateSet;
use crate::complete::context::ClauseKind;
use crate::complete::context::Context;
use crate::complete::context::Location;

/// Logical operators that chain predicates in a boolean clause.
const LOGICAL_OPERATORS: [&str; 2] = ["AND", "OR"];

/// Suggests logical operators (`AND`/`OR`) once a predicate is complete, so
/// `WHERE id = 1 ^` offers a way to extend the condition.
#[derive(Debug, Default)]
pub struct OperatorProvider;

impl Completer for OperatorProvider {
    fn complete<'a>(&mut self, _ctx: &mut Context<'a>, b: &mut CandidateSet<'a>) {
        for op in LOGICAL_OPERATORS {
            b.push(CandidateBuilder::operator(op).build());
        }
    }

    fn should_complete<'a>(&self, ctx: &Context<'a>) -> bool {
        // Only inside a boolean clause, and only after a complete sub-expression
        // (a value token followed by a space), never mid-expression.
        matches!(ctx.clause().kind, ClauseKind::Where)
            && matches!(
                &ctx.cursor().location,
                Location::Space(inner)
                    if matches!(**inner, Location::Ident | Location::Literal)
            )
    }
}
