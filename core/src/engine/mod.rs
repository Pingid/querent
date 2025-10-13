use crate::{
    catalog::{CatalogRead, CatalogResult},
    dialect::DialectSpec,
    doc::Content,
    lex::lex,
    parse::parse_statement_at_cursor,
};

mod completion;
mod context;
mod provider;
mod ranker;

pub use completion::*;
use context::build_context;
use ranker::{DefaultRanker, DefaultScorer, Ranker};

#[derive(Clone)]
pub struct Engine<C> {
    pub spec: &'static DialectSpec,
    pub catalog: C,
    pub ranker: DefaultRanker<DefaultScorer>,
}

impl<C: CatalogRead> Engine<C> {
    pub fn new(catalog: C, spec: &'static DialectSpec) -> Self {
        Self {
            spec,
            catalog,
            ranker: DefaultRanker::new(DefaultScorer),
        }
    }

    pub async fn complete(&self, doc: &Content) -> CatalogResult<Vec<Completion>> {
        complete(doc, &self.catalog, self.spec).await
    }
}

async fn complete<C: CatalogRead>(
    doc: &Content,
    catalog: &C,
    spec: &DialectSpec,
) -> CatalogResult<Vec<Completion>> {
    let txt = doc.to_string();
    let tokens = lex(spec, &txt);
    let Some(stmt) = parse_statement_at_cursor(&tokens, doc.cursor()) else {
        return Ok(vec![]);
    };
    let cursor = doc.cursor().min(txt.len());
    let ctx = build_context(&txt, &tokens, cursor, &stmt);
    let fragment = ctx.cursor.fragment.clone();
    let completions = provider::complete(&ctx, catalog, spec).await;
    Ok(DefaultRanker::new(DefaultScorer).rank(&fragment, completions?))
}
