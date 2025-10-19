use std::borrow::Cow;

use crate::{
    complete::{CompletionBuilder, Context},
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
                table_name: name.to_string(),
                schema_name: Some(schema.to_string()),
                data_type: schema::DataType::Text,
                is_nullable: Some(true),
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

    pub fn run_with(
        self,
        complete: impl Fn(&Context<'_>, &mut CompletionBuilder),
    ) -> CompletionTestResult {
        let spec = self.spec.unwrap_or_else(|| Ansi::default().spec.clone());
        let ctx = Context::build(&spec, &self.schema, &self.input, self.cursor).unwrap();
        let mut builder = CompletionBuilder::new();
        complete(&ctx, &mut builder);
        CompletionTestResult {
            input: self.input,
            schema: self.schema,
            completions: builder,
        }
    }
}

#[derive(Debug)]
pub struct CompletionTestResult {
    pub input: String,
    pub schema: schema::Cache,
    pub completions: CompletionBuilder,
}

impl CompletionTestResult {
    pub fn assert_labels(&self, expected: &[&str]) {
        let mut completions = self
            .completions
            .items
            .iter()
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>();
        completions.sort();
        let mut expected = expected.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        expected.sort();
        assert_eq!(completions, expected, "{}", self.input);
    }

    pub fn assert_all_schema_columns(&self, qualified: bool) {
        let schema_cols = self
            .schema
            .get_columns()
            .iter()
            .map(|c| match qualified {
                true => format!(
                    "{}.{}",
                    c.schema_name.clone().unwrap_or_default(),
                    c.column_name
                ),
                false => c.column_name.clone(),
            })
            .collect::<Vec<_>>();
        self.assert_labels(&schema_cols.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    }
}
