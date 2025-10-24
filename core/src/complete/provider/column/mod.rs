use crate::complete::completion::CompletionBuilder;
use crate::complete::context::Context;

mod helper;
mod select;
mod where_;

pub fn complete(ctx: &mut Context<'_>, builder: &mut CompletionBuilder) {
    select::complete(ctx, builder);
    where_::complete(ctx, builder);
}
