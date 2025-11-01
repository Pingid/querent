use std::borrow::Cow;

use crate::complete::completion::CompletionBuilder;
use crate::complete::completion::Completions;
use crate::complete::context::Context;
use crate::dialect::DialectSpec;
use crate::dialect::ansi;
use crate::lex::Token;
use crate::lex::lex;
use crate::schema;
use crate::schema::CacheBuilder;

pub fn get_caret_cursor<'a>(sql_with_caret: &'a str) -> (Cow<'a, str>, usize) {
    let pos = sql_with_caret.find('^');
    if let Some(pos) = pos {
        let (before, after_with_caret) = sql_with_caret.split_at(pos);
        let s = [before, &after_with_caret[1..]].concat(); // allocates once
        (Cow::Owned(s), pos)
    } else {
        (Cow::Borrowed(sql_with_caret), sql_with_caret.len())
    }
}

// This is a workaround to create 'static lifetimes for testing
// In reality, we leak memory here but it's fine for tests
pub fn leaky_static_caret_cursor(sql_with_caret: &str) -> (&'static str, usize) {
    let (text, pos) = get_caret_cursor(sql_with_caret);
    (Box::leak(text.to_string().into_boxed_str()), pos)
}

pub fn ansi_lex<'a>(sql: &'a str) -> Vec<Token<'a>> {
    lex(&ansi::SPEC, sql)
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
        self.with_schema(users_posts_schema())
    }

    pub fn run_with(
        self, complete: impl Fn(&mut Context<'_>, &mut CompletionBuilder),
    ) -> CompletionTestResult {
        let (input, cursor) = get_caret_cursor(&self.input);
        let spec = self.spec.as_ref();
        let mut ctx = match spec {
            Some(spec) => Context::build(spec, &self.schema, &input, cursor),
            None => Context::build(&ansi::SPEC, &self.schema, &input, cursor),
        }
        .unwrap();
        let mut builder = CompletionBuilder::new();
        complete(&mut ctx, &mut builder);
        CompletionTestResult {
            query: input.to_string(),
            completions: builder.build(&ctx),
        }
    }
}

pub fn users_posts_schema() -> schema::Cache {
    use schema::DataType::*;
    let schema = CacheBuilder::new()
        .table_in("users", "public", None)
        .column_in("id", Integer, "users", "public", None)
        .column_in("name", Text, "users", "public", None)
        .column_in("email", Text, "users", "public", None)
        .table_in("posts", "public", None)
        .column_in("id", Integer, "posts", "public", None)
        .column_in("title", Text, "posts", "public", None)
        .column_in("content", Text, "posts", "public", None)
        .build();
    schema
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
    pub fn assert_labels(&self, expected: &[&str]) {
        let labels = self.labels();
        if labels.len() < expected.len() {
            panic!(
                "\nquery: {:?}\nexpected atleast {} labels, got {}",
                self.query,
                expected.len(),
                labels.len()
            );
        }
        assert_eq!(
            expected,
            labels[..expected.len()].to_vec(),
            "\n query: {:?}",
            self.query
        );
    }

    pub fn assert_labels_contains(&self, expected: &[&str]) {
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

    pub fn assert_missing_labels(&self, expected: &[&str]) {
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
