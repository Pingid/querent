use crate::complete::{CompletionBuilder, Context};

mod helper;
mod select;
mod where_;

pub fn complete(ctx: &Context<'_>, builder: &mut CompletionBuilder) {
    select::complete(ctx, builder);
    where_::complete(ctx, builder);
}
