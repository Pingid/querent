use crate::{
    catalog::CatalogRead, dialect::DialectSpec, doc::Content, parse::parse_statement, token::lex,
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
    pub ranker: Box<dyn Ranker + Send + Sync>,
}

impl Engine {
    pub fn new(catalog: Box<dyn CatalogRead + Send + Sync>, spec: &'static DialectSpec) -> Self {
        Self {
            spec,
            catalog,
            ranker: Box::new(DefaultRanker::new(DefaultScorer)),
        }
    }

    pub async fn complete(&self, doc: &Content) -> Vec<Completion> {
        let txt = doc.current_statement();
        let spec = self.spec;
        let tokens = lex(spec, &txt);
        let Some(stmt) = parse_statement(&tokens) else {
            return vec![];
        };
        let cursor = doc.cursor().min(txt.len());
        let ctx = build_context(&txt, &tokens, cursor, &stmt);
        let fragment = ctx.cursor.fragment.clone();
        let completions = provider::complete(&ctx, &*self.catalog, spec).await;
        self.ranker.rank(&fragment, completions)
    }
}
