use crate::ast;
use crate::dialect::DialectSpec;
use crate::doc::Content;
use crate::lex::Token;
use crate::lex::lex;
use crate::parse::parse_statement_at_cursor;
use crate::schema;
use crate::span::Loc;

pub mod completion;
pub mod context;
pub mod provider;
mod providers;

pub struct Engine {
    pub spec: &'static DialectSpec,
    pub schema: schema::Cache,
}

impl Engine {
    pub fn new(spec: &'static DialectSpec, schema: schema::Cache) -> Self {
        Self { spec, schema }
    }

    pub fn complete(&self, doc: &Content) -> completion::Completions {
        complete(&self.spec, &self.schema, doc)
    }
}

pub fn complete(
    spec: &DialectSpec, schema: &schema::Cache, doc: &Content,
) -> completion::Completions {
    let text = doc.to_string();
    let cursor = doc.cursor().min(text.len());
    let mut builder = completion::CompletionBuilder::new();
    let Some(mut ctx) = context::Context::build(spec, schema, &text, cursor) else {
        return builder.empty();
    };
    provider::complete(&mut ctx, &mut builder);
    builder.build(&ctx)
}

struct ParseResult<'a> {
    text: &'a str,
    tokens: Vec<Token<'a>>,
    cursor: usize,
    stmt: Option<Loc<ast::Statement>>,
    spec: &'a DialectSpec,
    schema: &'a schema::Cache,
}

impl<'a> ParseResult<'a> {
    pub fn parse(
        text: &'a str, spec: &'a DialectSpec, schema: &'a schema::Cache, cursor: usize,
    ) -> Self {
        let tokens = lex(spec, text);
        let stmt = parse_statement_at_cursor(&tokens, cursor);
        Self {
            text,
            tokens,
            cursor,
            stmt,
            spec,
            schema,
        }
    }
}
