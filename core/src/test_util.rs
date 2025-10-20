use std::borrow::Cow;

use crate::{
    complete::{CompletionBuilder, CompletionKind, Completions, Context},
    dialect::{Ansi, DialectSpec, DialectSpecProvider},
    lex::{Token, lex},
    schema,
};

pub fn with_caret_cursor<'a>(sql_with_caret: &'a str) -> (Cow<'a, str>, usize) {
    let pos = sql_with_caret.find('^').expect("missing ^");
    let (before, after_with_caret) = sql_with_caret.split_at(pos);
    let s = [before, &after_with_caret[1..]].concat(); // allocates once
    (Cow::Owned(s), pos)
}

pub fn ansi_tokens<'a>(sql: &'a str) -> Vec<Token<'a>> {
    let dialect = Ansi::default();
    lex(dialect.get_spec(), sql)
}

pub struct SchemaCacheBuilder(schema::Cache);
impl SchemaCacheBuilder {
    pub(crate) fn new() -> Self {
        Self(schema::Cache::default())
    }
    pub(crate) fn add_table(mut self, schema: &str, name: &str, cols: &[&str]) -> Self {
        self.0.add_table(schema::Table {
            table_name: name.to_string(),
            schema_name: Some(schema.to_string()),
        });
        for c in cols {
            self.0.add_column(schema::Column {
                column_name: c.to_string(),
                table_name: Some(name.to_string()),
                schema_name: Some(schema.to_string()),
                data_type: schema::DataType::Text,
                is_nullable: None,
            });
        }
        self
    }
    pub(crate) fn build(self) -> schema::Cache {
        self.0
    }
}

#[derive(Debug)]
pub struct CompletionTest {
    input: String,
    cursor: usize,
    schema: schema::Cache,
    spec: Option<DialectSpec>,
}

impl CompletionTest {
    pub fn from_input(input: &str) -> Self {
        let (input, cursor) = with_caret_cursor(input);
        Self {
            spec: None,
            input: input.to_string(),
            cursor,
            schema: schema::Cache::default(),
        }
    }

    pub fn with_schema(mut self, schema: schema::Cache) -> Self {
        self.schema = schema;
        self
    }

    pub fn with_users_posts(self) -> Self {
        self.with_schema(
            SchemaCacheBuilder::new()
                .add_table("public", "users", &["id", "name", "email"])
                .add_table("public", "posts", &["id", "title", "content"])
                .build(),
        )
    }

    pub fn run_with(self, complete: impl Fn(&Context<'_>, &mut CompletionBuilder)) -> Completions {
        let spec = self.spec.unwrap_or_else(|| Ansi::default().spec.clone());
        let ctx = Context::build(&spec, &self.schema, &self.input, self.cursor).unwrap();
        let mut builder = CompletionBuilder::new();
        complete(&ctx, &mut builder);
        builder.build(&ctx)
    }
}

pub trait CompletionTestExt {
    fn assert_col<const N: usize>(&self, expected: [&str; N]);
}

impl CompletionTestExt for Completions {
    fn assert_col<const N: usize>(&self, expected: [&str; N]) {
        let labels: Vec<_> = self
            .items
            .iter()
            .filter(|c| matches!(c.kind, CompletionKind::Column(_)))
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels, expected);
    }
}
