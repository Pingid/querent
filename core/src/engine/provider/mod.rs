use crate::{
    catalog::{CatalogRead, CatalogResult},
    dialect::DialectSpec,
    engine::{Completion, context::Context},
};

mod column;
mod keyword;
mod operator;
mod table;

pub async fn complete<C: CatalogRead + ?Sized>(
    ctx: &Context,
    catalog: &C,
    spec: &DialectSpec,
) -> CatalogResult<Vec<Completion>> {
    let mut completions = Vec::new();
    if keyword::supports(ctx) {
        completions.extend(keyword::complete(ctx, spec));
    }
    if table::supports(ctx) {
        completions.extend(table::complete(ctx, catalog).await?);
    }
    if column::supports(ctx) {
        completions.extend(column::complete(ctx, catalog).await?);
    }
    // if operator::supports(ctx) {
    //     completions.extend(operator::complete(ctx, spec).await);
    // }
    Ok(completions)
}
