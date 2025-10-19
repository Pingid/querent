use super::completion::CompletionBuilder;
use super::context::Context;

mod column;
mod keyword;
mod operator;
mod table;

pub fn complete(ctx: &Context, builder: &mut CompletionBuilder) {
    keyword::complete(ctx, builder);
    table::complete(ctx, builder);
    column::complete(ctx, builder);
}
