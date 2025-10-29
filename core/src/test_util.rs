use std::borrow::Cow;

use crate::complete::completion::CompletionBuilder;
use crate::complete::completion::Completions;
use crate::complete::context::Context;
use crate::dialect::DialectSpec;
use crate::dialect::ansi;
use crate::lex::Token;
use crate::lex::lex;
use crate::schema;

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
pub fn get_leaky_static_caret_cursor(sql_with_caret: &str) -> (&'static str, usize) {
    let (text, pos) = get_caret_cursor(sql_with_caret);
    (Box::leak(text.to_string().into_boxed_str()), pos)
}

pub fn ansi_tokens<'a>(sql: &'a str) -> Vec<Token<'a>> {
    lex(&ansi::SPEC, sql)
}

pub struct SchemaCacheBuilder(schema::Cache);
impl SchemaCacheBuilder {
    pub(crate) fn new() -> Self {
        Self(schema::Cache::default())
    }

    pub(crate) fn add_function(
        mut self, schema: &str, name: &str, function_type: schema::FunctionType,
        params: &[schema::DataType], return_type: schema::DataType,
    ) -> Self {
        self.0.add_function(schema::Function {
            function_name: name.to_string(),
            parameter_types: params.to_vec(),
            function_type,
            description: None,
            schema_name: Some(schema.to_string()),
            database_name: None,
            return_type,
        });
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
        let mut schema = schema::Cache::default();
        // users table
        schema.add_table(schema::Table {
            table_name: "users".to_string(),
            schema_name: Some("public".to_string()),
            database_name: None,
            table_type: Some(schema::TableType::Table),
        });
        schema.add_column(schema::Column {
            column_name: "id".to_string(),
            table_name: Some("users".to_string()),
            schema_name: Some("public".to_string()),
            data_type: schema::DataType::Integer,
            is_nullable: None,
        });
        schema.add_column(schema::Column {
            column_name: "name".to_string(),
            table_name: Some("users".to_string()),
            schema_name: Some("public".to_string()),
            data_type: schema::DataType::Text,
            is_nullable: None,
        });
        schema.add_column(schema::Column {
            column_name: "email".to_string(),
            table_name: Some("users".to_string()),
            schema_name: Some("public".to_string()),
            data_type: schema::DataType::Text,
            is_nullable: None,
        });

        // posts table
        schema.add_table(schema::Table {
            table_name: "posts".to_string(),
            schema_name: Some("public".to_string()),
            database_name: None,
            table_type: Some(schema::TableType::Table),
        });
        schema.add_column(schema::Column {
            column_name: "id".to_string(),
            table_name: Some("posts".to_string()),
            schema_name: Some("public".to_string()),
            data_type: schema::DataType::Integer,
            is_nullable: None,
        });
        schema.add_column(schema::Column {
            column_name: "title".to_string(),
            table_name: Some("posts".to_string()),
            schema_name: Some("public".to_string()),
            data_type: schema::DataType::Text,
            is_nullable: None,
        });
        schema.add_column(schema::Column {
            column_name: "content".to_string(),
            table_name: Some("posts".to_string()),
            schema_name: Some("public".to_string()),
            data_type: schema::DataType::Text,
            is_nullable: None,
        });

        self.with_schema(schema)
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
