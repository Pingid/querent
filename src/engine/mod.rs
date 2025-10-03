use crate::{catalog::CatalogRead, dialect::Dialect, doc::Doc, parse::parse_statement, token::lex};

mod completion;
mod context;
mod provider;
mod ranker;

pub use completion::*;
use context::build_context;
use provider::ProviderRegistry;
use ranker::{DefaultRanker, DefaultScorer, Ranker};

pub struct Engine<C, D> {
    pub catalog: C,
    pub dialect: D,
    pub ranker: Box<dyn Ranker>,
    pub providers: ProviderRegistry,
}

impl<C, D> Engine<C, D>
where
    C: CatalogRead + Send + Sync,
    D: Dialect,
{
    pub fn new(catalog: C, dialect: D) -> Self {
        Self {
            catalog,
            dialect,
            ranker: Box::new(DefaultRanker::new(DefaultScorer)),
            providers: ProviderRegistry::default(),
        }
    }

    pub async fn complete(&self, doc: &Doc) -> Vec<Completion> {
        let txt = doc.current_statement();
        let spec = self.dialect.get_spec();
        let tokens = lex(spec, &txt);
        let Some(stmt) = parse_statement(&tokens) else {
            return vec![];
        };
        let cursor = doc.cursor().min(txt.len());
        let ctx = build_context(&txt, &tokens, cursor, &stmt);
        let fragment = ctx.cursor.fragment.clone();
        let completions = self.providers.complete(&self.catalog, spec, ctx).await;
        self.ranker.rank(&fragment, completions)
    }
}
