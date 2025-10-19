use crate::{
    complete::{Completion, context::Context},
    dialect::DialectSpec,
    schema,
};

mod column;
mod keyword;
mod operator;
mod table;

pub fn complete(ctx: &Context, spec: &DialectSpec, cache: &schema::Cache) -> Vec<Completion> {
    let mut completions = Vec::new();
    if keyword::supports(ctx) {
        completions.extend(keyword::complete(ctx, spec));
    }
    if table::supports(ctx) {
        completions.extend(table::complete(ctx, cache));
    }
    if column::supports(ctx) {
        completions.extend(column::complete(ctx, cache));
    }
    // if operator::supports(ctx) {
    //     completions.extend(operator::complete(ctx, spec));
    // }
    completions
}
