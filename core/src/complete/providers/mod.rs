use crate::complete::Completer;
use crate::complete::completion::CandidateSet;
use crate::complete::context::Context;
use crate::complete::providers::any::AnyProvider;

pub mod any;
pub mod column;

pub const DEFAULT_PROVIDERS: [AnyProvider; 1] =
    [AnyProvider::ColumnProvider(column::ColumnProvider)];

pub struct DefaultProviders;

impl<'a> Completer<'a> for DefaultProviders {
    fn complete(&mut self, ctx: &mut Context<'a>, b: &mut CandidateSet<'a>) {
        for provider in DEFAULT_PROVIDERS.iter_mut() {
            provider.complete(ctx, b);
        }
    }
}
