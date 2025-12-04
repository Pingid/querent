use std::fmt;

use itemize::IntoItems;
use smol_str::SmolStr;

use crate::complete::Completer;
#[cfg(test)]
use crate::complete::DefaultCompleter;
use crate::complete::candidate::Candidate;
use crate::complete::candidate::CandidateLineage;
use crate::complete::candidate::CandidateSet;
use crate::complete::context::Context;
use crate::complete::types::Completion;
use crate::complete::types::CompletionKind;
use crate::dialect;
use crate::schema;

#[derive(Debug, IntoItems)]
#[items_from(
    types(&'a str,schema::Cache, &'a dialect::DialectSpec, Assert),
    tuples(6),
    collections(vec, slice, array)
)]
pub enum ScenarioComp {
    Query(String, usize),
    Describe(String),
    Schema(schema::Cache),
    DialectSpec(dialect::DialectSpec),
    Completer(Box<dyn Completer>),
    Assert(Assert),
    Combined(Vec<ScenarioComp>),
}

impl Default for ScenarioComp {
    fn default() -> Self {
        Self::Combined(vec![])
    }
}

impl From<&str> for ScenarioComp {
    fn from(value: &str) -> Self {
        let (sql, cursor) = crate::test_utils::get_caret_cursor(value);
        Self::Query(sql.to_string(), cursor)
    }
}

impl From<schema::Cache> for ScenarioComp {
    fn from(value: schema::Cache) -> Self {
        Self::Schema(value)
    }
}

impl<'a> From<&'a dialect::DialectSpec> for ScenarioComp {
    fn from(value: &'a dialect::DialectSpec) -> Self {
        Self::DialectSpec(value.clone())
    }
}

impl<T: Completer + Default + 'static> From<T> for ScenarioComp {
    fn from(value: T) -> Self {
        Self::Completer(Box::new(value))
    }
}

impl From<Assert> for ScenarioComp {
    fn from(value: Assert) -> Self {
        Self::Assert(value)
    }
}

impl From<dialect::DialectSpec> for ScenarioComp {
    fn from(value: dialect::DialectSpec) -> Self {
        Self::DialectSpec(value.clone())
    }
}

#[derive(Debug, IntoItems)]
#[items_from(types(&'a str, &String), tuples(4), collections(vec, slice, array))]
pub struct Query(String);

impl<T> From<T> for Query
where T: Into<String>
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl ScenarioComp {
    pub fn query(mut self, query: impl IntoItems<Query>) -> Self {
        for value in query.into_items() {
            let (sql, cursor) = crate::test_utils::get_caret_cursor(&value.0);
            self = self.with(Self::Query(sql.to_string(), cursor))
        }
        self
    }

    pub fn describe(self, description: impl Into<String>) -> Self {
        self.with(Self::Describe(description.into()))
    }

    pub fn schema(self, schema: impl Into<schema::Cache>) -> Self {
        self.with(Self::Schema(schema.into()))
    }

    pub fn spec(self, spec: impl Into<dialect::DialectSpec>) -> Self {
        self.with(Self::DialectSpec(spec.into()))
    }

    pub fn completer<D: Completer + Default + 'static>(self, completer: D) -> Self {
        self.with(Self::Completer(Box::new(completer)))
    }

    pub fn assert(self, assert: impl Into<Assert>) -> Self {
        self.with(Self::Assert(assert.into()))
    }

    pub fn contains(self, completions: impl IntoItems<CompletionMatcher>) -> Self {
        self.with(Self::Assert(Assert::contains(completions)))
    }
    pub fn in_order(self, completions: impl IntoItems<CompletionMatcher>) -> Self {
        self.with(Self::Assert(Assert::InOrder(
            completions.into_items().collect(),
        )))
    }
    pub fn none_of(self, completions: impl IntoItems<CompletionMatcher>) -> Self {
        self.with(Self::Assert(Assert::NoneOf(
            completions.into_items().collect(),
        )))
    }
    pub fn starts(self, completions: impl IntoItems<CompletionMatcher>) -> Self {
        self.with(Self::Assert(Assert::Starts(
            completions.into_items().collect(),
        )))
    }

    pub fn and(self, other: impl IntoItems<ScenarioComp>) -> Self {
        self.with(other)
    }

    pub fn with(self, other: impl IntoItems<ScenarioComp>) -> Self {
        match self {
            Self::Combined(mut parts) => {
                parts.extend(other.into_items());
                Self::Combined(parts)
            }
            _ => {
                let mut items = vec![self];
                items.extend(other.into_items());
                Self::Combined(items)
            }
        }
    }

    #[track_caller]
    pub fn run(self) -> Vec<Completion> {
        let Self::Combined(mut parts) = self else {
            panic!("TestScenarioComponent::execute_test called on a non-combined component");
        };
        let mut sql_statements: Vec<(String, usize)> = vec![];
        let mut schema_cache = schema::Cache::default();
        let mut dialect_specs = vec![];
        let mut completers = vec![];
        let mut expected_matches = vec![];
        let mut descriptions = vec![];

        while let Some(component) = parts.pop() {
            match component {
                Self::Describe(description) => descriptions.push(description),
                Self::Query(sql, cursor) => sql_statements.push((sql, cursor)),
                Self::Schema(schema) => schema_cache = schema_cache.combine(schema),
                Self::DialectSpec(spec) => dialect_specs.push(spec),
                Self::Completer(completer) => completers.push(completer),
                Self::Assert(assertion) => expected_matches.push(assertion),
                Self::Combined(nested_parts) => parts.extend(nested_parts),
            }
        }

        let mut results = vec![];

        for dialect_spec in dialect_specs {
            for (sql_query, cursor_position) in sql_statements.iter() {
                let mut context =
                    Context::build(&dialect_spec, &schema_cache, &sql_query, *cursor_position)
                        .unwrap();
                let mut candidate_set = CandidateSet::new();
                for completer in completers.iter_mut() {
                    if completer.should_complete(&context) {
                        completer.complete(&mut context, &mut candidate_set);
                    }
                }
                candidate_set.sort();
                let candidate_items = candidate_set.items.clone();

                let completions = candidate_set.completions().items;
                for assertion in expected_matches.iter() {
                    if let Err(error) = assertion.verify(&completions) {
                        panic!(
                            "\n{}",
                            Self::generate_report(
                                &error,
                                sql_query,
                                &descriptions,
                                &completions,
                                &candidate_items
                            )
                        );
                    }
                }
                results.extend(completions);
            }
        }

        results
    }

    pub fn generate_report(
        error: &String, sql_query: &str, descriptions: &[String], completions: &[Completion],
        candidates: &[Candidate<'_>],
    ) -> String {
        use docloom::prelude::*;
        use docloom::term;
        let max_completions_to_show = 20;
        let shown = completions.len().min(max_completions_to_show);
        let hidden = completions.len().saturating_sub(max_completions_to_show);

        let lineage_table = Self::lineage_table(candidates);
        let doc2 = term::doc((
            "Test Failure".h1(),
            p(descriptions),
            error.h3(),
            hr(),
            quote(sql_query.bold()),
            hr(),
            "Completions".h2(),
            p(("Showing ", shown, " of ", completions.len(), " completions")),
            lineage_table,
            format!("... ({} more completions not shown)", hidden).italic(),
        ));

        let style = term::Style::default().heading_colors([
            term::Style::RED,
            term::Style::GREEN,
            term::Style::YELLOW,
            term::Style::BLUE,
            term::Style::MAGENTA,
            term::Style::CYAN,
        ]);

        format!("{}", doc2.with_style(style))
    }

    fn lineage_table(candidates: &[Candidate<'_>]) -> docloom::Block {
        use docloom::prelude::table;
        let mut rows = vec![];
        for candidate in candidates.iter() {
            let lineage = candidate.lineage.borrow();
            let lineage_summary = lineage
                .iter()
                .filter_map(|line| match line {
                    CandidateLineage::Ranked(ranker_name, score) if *score > 0.0 => {
                        let cleaned_name = match ranker_name.split_once('(') {
                            Some((prefix, _)) => prefix.to_string(),
                            None => ranker_name.clone(),
                        };
                        Some(format!("{} ({:.1})", cleaned_name, score))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            rows.push(vec![
                format!("{:?}", candidate.completion.kind),
                candidate.completion.label.to_string(),
                lineage_summary.join(" • "),
            ]);
        }
        table(("Kind", "Label", "Lineage"), rows)
    }
}

#[derive(Debug, Clone)]
pub enum Assert {
    Starts(Vec<CompletionMatcher>),
    Contains(Vec<CompletionMatcher>),
    InOrder(Vec<CompletionMatcher>),
    NoneOf(Vec<CompletionMatcher>),
    Combined(Vec<Assert>),
}

impl Assert {
    pub fn contains(completions: impl IntoItems<CompletionMatcher>) -> Self {
        Self::Contains(completions.into_items().collect())
    }
    pub fn starts(completions: impl IntoItems<CompletionMatcher>) -> Self {
        Self::Starts(completions.into_items().collect())
    }
    pub fn in_order(completions: impl IntoItems<CompletionMatcher>) -> Self {
        Self::InOrder(completions.into_items().collect())
    }
    pub fn none_of(completions: impl IntoItems<CompletionMatcher>) -> Self {
        Self::NoneOf(completions.into_items().collect())
    }
    pub fn matches(&self, completions: &[Completion]) -> bool {
        match self {
            Assert::Starts(matchers) => {
                for (index, matcher) in matchers.iter().enumerate() {
                    let Some(completion) = completions.get(index) else {
                        return false;
                    };
                    if !matcher.matches(completion) {
                        return false;
                    }
                }
                true
            }
            Assert::Contains(matchers) => {
                for matcher in matchers {
                    if !completions
                        .iter()
                        .any(|completion| matcher.matches(completion))
                    {
                        return false;
                    }
                }
                true
            }
            Assert::InOrder(matchers) => {
                let mut previous_index = None;
                for matcher in matchers {
                    let current_index = completions
                        .iter()
                        .position(|completion| matcher.matches(completion));
                    let Some(current_index) = current_index else {
                        return false;
                    };
                    if let Some(prev_idx) = previous_index
                        && current_index < prev_idx
                    {
                        return false;
                    }
                    previous_index = Some(current_index);
                }
                true
            }
            Assert::NoneOf(matchers) => {
                for matcher in matchers {
                    if completions
                        .iter()
                        .any(|completion| matcher.matches(completion))
                    {
                        return false;
                    }
                }
                true
            }
            Assert::Combined(assertions) => {
                for assertion in assertions {
                    if !assertion.matches(completions) {
                        return false;
                    }
                }
                true
            }
        }
    }
    pub fn verify(&self, completions: &[Completion]) -> Result<(), String> {
        if self.matches(completions) {
            return Ok(());
        }
        Err(self.format_expected())
    }

    fn format_expected(&self) -> String {
        let show = |matchers: &[CompletionMatcher]| {
            matchers
                .iter()
                .map(|matcher| matcher.to_string())
                .collect::<Vec<_>>()
                .join("\n  ")
        };
        match self {
            Assert::Starts(m) => {
                format!("Expected completions to start with:\n  {}", show(m))
            }
            Assert::Contains(m) => {
                format!("Expected completions to contain:\n  {}", show(m))
            }
            Assert::InOrder(m) => {
                format!("Expected order to match:\n  {}", show(m))
            }
            Assert::NoneOf(m) => {
                format!("Expected completions to not contain:\n  {}", show(m))
            }
            Assert::Combined(assertions) => {
                format!("Expected {:?}", assertions)
            }
        }
    }
}

#[derive(Debug, Clone, IntoItems)]
#[items_from(types(&str, CompletionKind), tuples(4), collections(vec, slice, array))]
pub enum CompletionMatcher {
    Label(SmolStr),
    InsertText(SmolStr),
    Kind(CompletionKind),
    AllOf(Vec<CompletionMatcher>),
}

impl From<&str> for CompletionMatcher {
    fn from(value: &str) -> Self {
        Self::Label(value.into())
    }
}

impl From<CompletionKind> for CompletionMatcher {
    fn from(value: CompletionKind) -> Self {
        Self::Kind(value)
    }
}

impl CompletionMatcher {
    pub fn matches(&self, completion: &Completion) -> bool {
        match self {
            Self::Label(label) => label == &completion.label,
            Self::InsertText(insert_text) => insert_text == &completion.insert_text,
            Self::Kind(kind) => kind == &completion.kind,
            Self::AllOf(m) => m.iter().all(|matcher| matcher.matches(completion)),
        }
    }
}

impl fmt::Display for CompletionMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Label(label) => write!(f, "label = {:?}", label),
            Self::InsertText(text) => write!(f, "insert_text = {:?}", text),
            Self::Kind(kind) => write!(f, "kind = {:?}", kind),
            Self::AllOf(matchers) => {
                write!(f, "(")?;
                for (i, matcher) in matchers.iter().enumerate() {
                    if i > 0 {
                        write!(f, " && ")?;
                    }
                    write!(f, "{matcher}")?;
                }
                write!(f, ")")
            }
        }
    }
}

#[macro_export]
macro_rules! completions {
    ($base:expr, { $($sql:expr => { $($key:ident: [$($item:tt),* $(,)?]),* $(,)? });* $(;)? }) => {
        {
            $(
                let matches = vec![
                    $($crate::completions_match_type!($key, [$($item),*])),*
                ];
                let match_set = if matches.is_empty() {
                    $crate::test_utils::Assert::Combined(vec![])
                } else if matches.len() == 1 {
                    matches.into_iter().next().unwrap()
                } else {
                    $crate::test_utils::Assert::Combined(matches)
                };
                let scenario = $base
                    .with($sql)
                    .with(match_set);
                scenario.run();
            )*
        }
    };
}

#[macro_export]
macro_rules! completions_match_type {
    (none_of, [$($item:tt),* $(,)?]) => {
        $crate::match_set!(NoneOf($($item),*))
    };
    (contains, [$($item:tt),* $(,)?]) => {
        $crate::match_set!(Contains($($item),*))
    };
    (in_order, [$($item:tt),* $(,)?]) => {
        $crate::match_set!(InOrder($($item),*))
    };
}

#[macro_export]
macro_rules! match_set {
    (Empty) => { crate::test_utils::Assert::Empty };
    (NotEmpty) => { crate::test_utils::Assert::NotEmpty };
    ($v:ident) => { crate::test_utils::Assert::$v };
    ($v:ident($($rest:tt),* $(,)?)) => { crate::test_utils::Assert::$v(vec![$( $crate::completion_match!{ $rest } ),*]) };
    ( $($k:ident($($rest:tt)*)),* $(,)? ) => {{
        crate::test_utils::Assert::Combined(vec![$( $crate::match_set!($k($($rest)*)) ),*])
    }};
}

#[macro_export]
macro_rules! completion_match {
    ({ $($k:ident : $v:tt),* $(,)? }) => { crate::test_utils::CompletionMatcher::AllOf(vec![$( $crate::completion_match!($k : $v) ),*]) };
    (label: $v:expr) => { crate::test_utils::CompletionMatcher::Label($v.into()) };
    (insert_text: $v:expr) => { crate::test_utils::CompletionMatcher::InsertText($v.into()) };
    (kind: $v:ident) => { crate::test_utils::CompletionMatcher::Kind(crate::complete::types::CompletionKind::$v) };
    ($v:ident) => { crate::test_utils::CompletionMatcher::Kind(crate::complete::types::CompletionKind::$v) };
    ($v:expr) => { crate::test_utils::CompletionMatcher::Label($v.into()) };
    ($i:ident: $v:tt) => {{
        compile_error!(concat!("unknown completion field name: '", stringify!($i), "', expected one of: 'label', 'insert_text', 'kind'"));
        crate::test_utils::CompletionMatcher::AllOf(vec![])
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::posts_schema;

    fn create_base_scenario() -> ScenarioComp {
        use crate::test_utils::users_schema;

        ScenarioComp::default()
            .completer(DefaultCompleter::default())
            .with(users_schema())
            .with(posts_schema())
            .with(&dialect::ansi::SPEC)
    }

    #[test]
    fn test_macro_syntax() {
        completions!(create_base_scenario(), {
            "SELECT ^ FROM users" => { in_order: ["id", "name"] };
            "SELECT ^ FROM posts" => { in_order: ["id", "title"] };
        });
    }
}
