use std::borrow::Cow;

use crate::{
    complete::{CompletionBuilder, Completions, Context},
    dialect::{Ansi, DialectSpec, DialectSpecProvider},
    lex::{Token, lex},
    schema,
};

pub fn with_caret_cursor<'a>(sql_with_caret: &'a str) -> (Cow<'a, str>, usize) {
    let pos = sql_with_caret.find('^');
    if let Some(pos) = pos {
        let (before, after_with_caret) = sql_with_caret.split_at(pos);
        let s = [before, &after_with_caret[1..]].concat(); // allocates once
        (Cow::Owned(s), pos)
    } else {
        (Cow::Borrowed(sql_with_caret), sql_with_caret.len())
    }
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
            database_name: None,
            table_type: None,
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
    schema: schema::Cache,
    spec: Option<DialectSpec>,
}

impl CompletionTest {
    pub fn from_input(input: &str) -> Self {
        Self {
            spec: None,
            input: input.to_string(),
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
        let (input, cursor) = with_caret_cursor(&self.input);
        let spec = self.spec.unwrap_or_else(|| Ansi::default().spec.clone());
        let ctx = Context::build(&spec, &self.schema, &input, cursor).unwrap();
        let mut builder = CompletionBuilder::new();
        complete(&ctx, &mut builder);
        CompletionTestResult {
            query: input.to_string(),
            completions: builder.build(&ctx),
        }
    }
}

#[derive(Debug)]
pub struct CompletionTestResult {
    query: String,
    completions: Completions,
}

impl CompletionTestResult {
    pub fn labels(&self) -> Vec<&str> {
        self.completions
            .items
            .iter()
            .map(|c| c.label.as_str())
            .collect()
    }
    pub fn assert_complete<const N: usize>(&self, expected: &str) {
        let completion = &self.completions.items[N];
        let replace = completion.replace;
        let next = format!(
            "{}{}{}",
            &self.query[..replace.start],
            completion.insert_text,
            &self.query[replace.end..]
        );
        assert_eq!(next, expected);
    }
    pub fn assert_labels<const N: usize>(&self, expected: [&str; N]) {
        let labels = self.labels();
        if labels.len() < N {
            panic!(
                "\nquery: {:?}\nexpected atleast {} labels, got {}",
                self.query,
                N,
                labels.len()
            );
        }
        assert_eq!(expected, labels[..N], "\n query: {:?}", self.query);
    }

    pub fn assert_labels_contains<const N: usize>(&self, expected: [&str; N]) {
        let labels = self.labels();
        for label in expected {
            assert!(labels.contains(&label), "label {} should be present", label);
        }
    }

    pub fn assert_empty(&self) {
        assert!(
            self.completions.items.len() == 0,
            "\nquery: {:?}\nexpected no completions, got {:?}",
            self.query,
            self.labels()
        );
    }

    pub fn assert_not_empty(&self) {
        assert!(
            self.completions.items.len() > 0,
            "\nquery: {:?}\nexpected completions, got none",
            self.query
        );
    }

    pub fn assert_missing_labels<const N: usize>(&self, expected: [&str; N]) {
        let labels = self.labels();
        for label in expected {
            assert!(
                !labels.contains(&label),
                "label {} should be missing",
                label
            );
        }
    }
}
