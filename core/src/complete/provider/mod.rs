use super::completion::CompletionBuilder;
use super::context::Context;

mod column;
mod function;
mod keyword;
mod operator;
mod table;

pub fn complete(ctx: &mut Context<'_>, builder: &mut CompletionBuilder) {
    keyword::complete(ctx, builder);
    table::complete(ctx, builder);
    column::complete(ctx, builder);
    operator::complete(ctx, builder);
    function::complete(ctx, builder);
}
