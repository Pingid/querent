use crate::{
    catalog::{CatalogRead, CatalogResult},
    dialect::DialectSpec,
    doc::Content,
    parse::parse_statement,
    token::lex,
};

mod completion;
mod context;
mod provider;
mod ranker;

pub use completion::*;
use context::build_context;
use ranker::{DefaultRanker, DefaultScorer, Ranker};

pub struct Engine {
    pub spec: &'static DialectSpec,
    pub catalog: Box<dyn CatalogRead + Send + Sync>,
    pub ranker: DefaultRanker<DefaultScorer>,
}

impl Engine {
    pub fn new(catalog: Box<dyn CatalogRead + Send + Sync>, spec: &'static DialectSpec) -> Self {
        Self {
            spec,
            catalog,
            ranker: DefaultRanker::new(DefaultScorer),
        }
    }

    pub async fn complete(&self, doc: &Content) -> CatalogResult<Vec<Completion>> {
        complete(doc, &*self.catalog, self.spec).await
    }
}

async fn complete<C: CatalogRead + ?Sized>(
    doc: &Content,
    catalog: &C,
    spec: &DialectSpec,
) -> CatalogResult<Vec<Completion>> {
    let txt = doc.current_statement();
    let tokens = lex(spec, &txt);
    let Some(stmt) = parse_statement(&tokens) else {
        return Ok(vec![]);
    };
    let cursor = doc.cursor().min(txt.len());
    let ctx = build_context(&txt, &tokens, cursor, &stmt);
    let fragment = ctx.cursor.fragment.clone();
    let completions = provider::complete(&ctx, catalog, spec).await?;
    Ok(DefaultRanker::new(DefaultScorer).rank(&fragment, completions))
}
