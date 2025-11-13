use std::fmt;

use smol_str::SmolStr;

use crate::{
    complete::{
        Completer,
        candidate::{Candidate, CandidateLineage, CandidateSet},
        context::Context,
        types::{Completion, CompletionKind},
    },
    dialect, schema,
};

#[cfg(test)]
use crate::complete::DefaultCompleter;

#[derive(Debug)]
pub enum TestScenarioComponent {
    SqlQuery(String, usize),
    SchemaDefinition(schema::Cache),
    DialectSpec(dialect::DialectSpec),
    CompleterImplementation(Box<dyn Completer>),
    ExpectedMatches(CompletionAssertion),
    Combined(Vec<TestScenarioComponent>),
}

impl Default for TestScenarioComponent {
    fn default() -> Self {
        Self::Combined(vec![])
    }
}

impl From<&str> for TestScenarioComponent {
    fn from(value: &str) -> Self {
        let (sql, cursor) = crate::test_utils::get_caret_cursor(value);
        Self::SqlQuery(sql.to_string(), cursor)
    }
}

impl From<schema::Cache> for TestScenarioComponent {
    fn from(value: schema::Cache) -> Self {
        Self::SchemaDefinition(value)
    }
}

impl From<&'static dialect::DialectSpec> for TestScenarioComponent {
    fn from(value: &'static dialect::DialectSpec) -> Self {
        Self::DialectSpec(value.clone())
    }
}

impl<T: Completer + Default + 'static> From<T> for TestScenarioComponent {
    fn from(value: T) -> Self {
        Self::CompleterImplementation(Box::new(value))
    }
}

impl From<CompletionAssertion> for TestScenarioComponent {
    fn from(value: CompletionAssertion) -> Self {
        Self::ExpectedMatches(value)
    }
}

impl TestScenarioComponent {
    pub fn and(self, other: impl Into<Self>) -> Self {
        match self {
            Self::Combined(mut parts) => {
                parts.push(other.into());
                Self::Combined(parts)
            }
            _ => Self::Combined(vec![self, other.into()]),
        }
    }

    #[track_caller]
    pub fn execute_test(self) -> Vec<Completion> {
        let Self::Combined(mut parts) = self else {
            panic!("TestScenarioComponent::execute_test called on a non-combined component");
        };
        let mut sql_statements: Vec<(String, usize)> = vec![];
        let mut schema_cache = schema::Cache::default();
        let mut dialect_specs = vec![];
        let mut completers = vec![];
        let mut expected_matches = vec![];

        while let Some(component) = parts.pop() {
            match component {
                Self::SqlQuery(sql, cursor) => sql_statements.push((sql, cursor)),
                Self::SchemaDefinition(schema) => schema_cache = schema_cache.combine(schema),
                Self::DialectSpec(spec) => dialect_specs.push(spec),
                Self::CompleterImplementation(completer) => completers.push(completer),
                Self::ExpectedMatches(assertion) => expected_matches.push(assertion),
                Self::Combined(nested_parts) => parts.extend(nested_parts),
            }
        }

        let mut results = vec![];

        for dialect_spec in dialect_specs {
            for (sql_query, cursor_position) in sql_statements.iter() {
                let mut context =
                    Context::build(&dialect_spec, &schema_cache, &sql_query, *cursor_position)
                        .unwrap();
                let mut candidate_set = CandidateSet::default();
                for completer in completers.iter_mut() {
                    if completer.should_complete(&context) {
                        completer.complete(&mut context, &mut candidate_set);
                    }
                }
                let candidate_items = candidate_set.items.clone();
                let completions = candidate_set.completions().items;
                for assertion in expected_matches.iter() {
                    if let Err(error) = assertion.verify(&completions) {
                        panic!(
                            "\n{}",
                            error.generate_report(sql_query, &completions, &candidate_items)
                        );
                    }
                }
                results.extend(completions);
            }
        }

        results
    }
}

#[derive(Debug, Clone)]
pub enum CompletionAssertion {
    Contains(Vec<CompletionMatcher>),
    InOrder(Vec<CompletionMatcher>),
    NoneOf(Vec<CompletionMatcher>),
    Combined(Vec<CompletionAssertion>),
}

impl CompletionAssertion {
    pub fn verify(&self, completions: &[Completion]) -> Result<(), TestFailure> {
        match self {
            CompletionAssertion::Contains(matchers) => {
                for matcher in matchers {
                    if !completions
                        .iter()
                        .any(|completion| matcher.matches(completion))
                    {
                        return Err(TestFailure::missing_expected(matcher));
                    }
                }
                Ok(())
            }
            CompletionAssertion::InOrder(matchers) => {
                let mut previous_index = None;
                for matcher in matchers {
                    let current_index = completions
                        .iter()
                        .position(|completion| matcher.matches(completion));
                    let Some(current_index) = current_index else {
                        return Err(TestFailure::missing_expected(matcher));
                    };
                    if let Some(prev_idx) = previous_index
                        && current_index < prev_idx
                    {
                        return Err(TestFailure::incorrect_order(
                            &completions[prev_idx],
                            &completions[current_index],
                        ));
                    }
                    previous_index = Some(current_index);
                }
                Ok(())
            }
            CompletionAssertion::NoneOf(matchers) => {
                for matcher in matchers {
                    if let Some(completion) = completions
                        .iter()
                        .find(|completion| matcher.matches(completion))
                    {
                        return Err(TestFailure::unexpected_completion(completion));
                    }
                }
                Ok(())
            }
            CompletionAssertion::Combined(assertions) => {
                for assertion in assertions {
                    assertion.verify(completions)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum CompletionMatcher {
    Label(SmolStr),
    InsertText(SmolStr),
    Kind(CompletionKind),
    AllOf(Vec<CompletionMatcher>),
}

impl CompletionMatcher {
    pub fn matches(&self, completion: &Completion) -> bool {
        match self {
            Self::Label(expected_label) => expected_label == &completion.label,
            Self::InsertText(expected_text) => expected_text == &completion.insert_text,
            Self::Kind(expected_kind) => expected_kind == &completion.kind,
            Self::AllOf(matchers) => matchers.iter().all(|matcher| matcher.matches(completion)),
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

pub enum TestFailure {
    MissingExpected(CompletionMatcher),
    UnexpectedCompletion(Completion),
    IncorrectOrder(Completion, Completion),
}

impl TestFailure {
    pub fn missing_expected(matcher: &CompletionMatcher) -> Self {
        Self::MissingExpected(matcher.clone())
    }
    pub fn unexpected_completion(completion: &Completion) -> Self {
        Self::UnexpectedCompletion(completion.clone())
    }
    fn incorrect_order(before: &Completion, after: &Completion) -> Self {
        Self::IncorrectOrder(before.clone(), after.clone())
    }
    pub fn generate_report(
        &self, sql_query: &str, completions: &[Completion], candidates: &[Candidate<'_>],
    ) -> String {
        use docloom::prelude::*;
        use docloom::term;
        let max_completions_to_show = 20;
        let shown = completions.len().min(max_completions_to_show);
        let hidden = completions.len().saturating_sub(max_completions_to_show);

        let lineage_table = Self::lineage_table(candidates);
        let doc2 = term::doc((
            "Test Failure".h1(),
            self.to_string().h3(),
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

impl fmt::Display for TestFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestFailure::MissingExpected(matcher) => {
                write!(f, "missing completion matching: {matcher}")
            }
            TestFailure::UnexpectedCompletion(completion) => write!(
                f,
                "unexpected completion: label={:?}, insert_text={:?}, kind={:?}",
                completion.label, completion.insert_text, completion.kind
            ),
            TestFailure::IncorrectOrder(before, after) => write!(
                f,
                "completions are out of order: {:?} appears after {:?}",
                before.label, after.label
            ),
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
                    $crate::test_utils::CompletionAssertion::Combined(vec![])
                } else if matches.len() == 1 {
                    matches.into_iter().next().unwrap()
                } else {
                    $crate::test_utils::CompletionAssertion::Combined(matches)
                };
                let scenario = $base
                    .and($sql)
                    .and(match_set);
                scenario.execute_test();
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
    (Empty) => { crate::test_utils::CompletionAssertion::Empty };
    (NotEmpty) => { crate::test_utils::CompletionAssertion::NotEmpty };
    ($v:ident) => { crate::test_utils::CompletionAssertion::$v };
    ($v:ident($($rest:tt),* $(,)?)) => { crate::test_utils::CompletionAssertion::$v(vec![$( $crate::completion_match!{ $rest } ),*]) };
    ( $($k:ident($($rest:tt)*)),* $(,)? ) => {{
        crate::test_utils::CompletionAssertion::Combined(vec![$( $crate::match_set!($k($($rest)*)) ),*])
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
    use crate::test_utils::posts_schema;

    use super::*;

    fn create_base_scenario() -> TestScenarioComponent {
        use crate::test_utils::users_schema;

        TestScenarioComponent::default()
            .and(DefaultCompleter::default())
            .and(users_schema())
            .and(posts_schema())
            .and(&dialect::ansi::SPEC)
    }

    #[test]
    fn test_column_completions() {
        completions!(create_base_scenario(), {
            "SELECT ^ FROM users" => { in_order: ["id", "name"] };
            "SELECT ^ FROM posts" => { in_order: ["id", "title"] };
        });
    }
}
