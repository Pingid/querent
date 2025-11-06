use crate::complete::Completer;
use crate::complete::candidate::CandidateSet;
use crate::complete::context::Context;

mod column;
pub use column::*;

pub struct DefaultProviders;

impl<'a> Completer<'a> for DefaultProviders {
    fn complete(&mut self, ctx: &mut Context<'a>, b: &mut CandidateSet<'a>) {
        ColumnProvider.complete(ctx, b);
    }
}
