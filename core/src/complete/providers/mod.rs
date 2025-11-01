use crate::complete::completion::CandidateSet;
use crate::complete::context::Context;

mod column;

pub fn complete<'a>(ctx: &mut Context<'a>, builder: &mut CandidateSet<'a>) {
    column::complete(ctx, builder);
}
