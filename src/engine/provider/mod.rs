use std::{
    future::{Future, ready},
    pin::Pin,
};

use crate::{
    dialect::DialectSpec,
    engine::{Completion, context::Context},
};

mod column;
mod keyword;
mod operator;
mod table;

use column::ColumnProvider;
use keyword::KeywordProvider;
use operator::OperatorProvider;
use table::TableProvider;

use crate::catalog::CatalogRead;

/// Providers return raw (unranked) completions for a given context.
pub trait CompletionProvider {
    fn supports(&self, ctx: &Context) -> bool;
    fn complete<'a>(
        &'a self,
        catalog: &'a (dyn CatalogRead + Send + Sync),
        spec: &'a DialectSpec,
        ctx: &'a Context,
    ) -> Pin<Box<dyn Future<Output = Vec<Completion>> + Send + 'a>>;
}

pub trait CompletionProviderSync {
    fn supports(&self, ctx: &Context) -> bool;
    fn complete(
        &self,
        catalog: &(dyn CatalogRead + Send + Sync),
        spec: &DialectSpec,
        ctx: &Context,
    ) -> Vec<Completion>;
}

impl<T: CompletionProviderSync> CompletionProvider for T {
    fn supports(&self, ctx: &Context) -> bool {
        self.supports(ctx)
    }
    fn complete<'a>(
        &'a self,
        catalog: &'a (dyn CatalogRead + Send + Sync),
        spec: &'a DialectSpec,
        ctx: &'a Context,
    ) -> Pin<Box<dyn Future<Output = Vec<Completion>> + Send + 'a>> {
        Box::pin(ready(self.complete(catalog, spec, ctx)))
    }
}

/// Aggregates providers and runs the ones that support the current context.
pub struct ProviderRegistry {
    list: Vec<Box<dyn CompletionProvider + Send + Sync>>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self {
            list: vec![
                Box::new(KeywordProvider),
                Box::new(TableProvider),
                Box::new(ColumnProvider),
                Box::new(OperatorProvider),
            ],
        }
    }
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { list: Vec::new() }
    }

    pub async fn complete(
        &self,
        catalog: &(dyn CatalogRead + Send + Sync),
        spec: &DialectSpec,
        ctx: Context,
    ) -> Vec<Completion> {
        let mut completions = Vec::new();
        for provider in &self.list {
            if provider.supports(&ctx) {
                completions.extend(provider.complete(catalog, spec, &ctx).await);
            }
        }
        completions
    }
}
