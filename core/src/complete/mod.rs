use crate::{
    dialect::DialectSpec, doc::Content, lex::lex, parse::parse_statement_at_cursor, schema,
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
    pub schema: schema::Cache,
}

impl Engine {
    pub fn new(spec: &'static DialectSpec, schema: schema::Cache) -> Self {
        Self { spec, schema }
    }

    pub fn complete(&self, doc: &Content) -> Vec<Completion> {
        complete(&self.spec, &self.schema, doc)
    }
}

pub fn complete(spec: &DialectSpec, schema: &schema::Cache, doc: &Content) -> Vec<Completion> {
    let txt = doc.to_string();
    let tokens = lex(spec, &txt);
    let Some(stmt) = parse_statement_at_cursor(&tokens, doc.cursor()) else {
        return vec![];
    };
    let cursor = doc.cursor().min(txt.len());
    let ctx = build_context(&txt, &tokens, cursor, &stmt);
    let completions = provider::complete(&ctx, spec, schema);
    DefaultRanker::new(DefaultScorer).rank(&ctx.cursor.fragment, completions)
}
