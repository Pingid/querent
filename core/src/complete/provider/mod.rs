use crate::complete::Completer;
use crate::complete::candidate::CandidateSet;
use crate::complete::context::Context;

mod column;
mod function;
mod keyword;
mod table;

pub use column::*;
pub use function::*;
pub use keyword::*;
pub use table::*;

#[derive(Default)]
pub struct DefaultProviders;

impl<'a> Completer<'a> for DefaultProviders {
    fn complete(&mut self, ctx: &mut Context<'a>, b: &mut CandidateSet<'a>) {
        if ColumnProvider.should_complete(ctx) {
            ColumnProvider.complete(ctx, b);
        }
        if FunctionProvider.should_complete(ctx) {
            FunctionProvider.complete(ctx, b);
        }
        if KeywordProvider.should_complete(ctx) {
            KeywordProvider.complete(ctx, b);
        }
        if TableProvider.should_complete(ctx) {
            TableProvider.complete(ctx, b);
        }
    }
}
