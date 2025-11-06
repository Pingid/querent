use crate::complete::Completer;
use crate::complete::candidate::{CandidateLineage, CandidateSet};
use crate::complete::context::Context;
use crate::complete::types::Completion;
use crate::dialect::DialectSpec;
use crate::dialect::{self};
use crate::schema;

mod matcher;
mod schemas;
mod util;

pub use matcher::*;
pub use schemas::*;
pub use util::*;

/// A test scenario for validating SQL completion functionality.
///
/// This struct encapsulates all the components needed to test SQL completions,
/// including the SQL queries, expected results, database schemas, dialect specifications,
/// and completion providers.
pub struct TestScenario {
    values: Vec<(String, usize)>,
    schema: schema::Cache,
    specs: Vec<DialectSpec>,
    expect: Expect,
}

impl TestScenario {
    pub fn new() -> Self {
        Self {
            values: vec![],
            schema: schema::Cache::default(),
            specs: vec![dialect::ansi::SPEC.clone()],
            expect: Expect::new(),
        }
    }
    pub fn value(mut self, value: (String, usize)) -> Self {
        self.values.push(value);
        self
    }
    pub fn schemas(mut self, schemas: Vec<schema::Cache>) -> Self {
        for schema in schemas {
            self.schema = self.schema.combine(schema);
        }
        self
    }
    pub fn specs(mut self, specs: Vec<DialectSpec>) -> Self {
        self.specs.extend(specs);
        self
    }
    pub fn expect(mut self, e: Expect) -> Self {
        self.expect = e;
        self
    }

    #[track_caller]
    pub fn run<'a>(&'a self, mut completer: impl Completer<'a> + 'a) -> Vec<Completion> {
        let mut results = vec![];
        for (sql, cursor) in &self.values {
            let schema = &self.schema;
            for spec in &self.specs {
                let mut ctx = Context::build(&spec, &schema, &sql, *cursor).unwrap();
                let mut candidates = CandidateSet::default();
                if completer.should_complete(&ctx) {
                    completer.complete(&mut ctx, &mut candidates);
                }
                let lineage = candidates
                    .items
                    .iter()
                    .map(|c| {
                        (
                            c.completion.label.to_string(),
                            c.score.0,
                            c.lineage.borrow().clone(),
                        )
                    })
                    .collect::<Vec<_>>();

                let lineage_fmt = lineage
                    .iter()
                    .filter(|(_, _, lines)| !lines.is_empty())
                    .map(|(label, score, lines)| {
                        format!(
                            "{:<20} {:>4}: {}",
                            label
                                .replace("Rank", "")
                                .replace("\"", "")
                                .replace("'", "")
                                .replace("<a>", ""),
                            score,
                            lines
                                .iter()
                                .rev()
                                .filter_map(|line| match line {
                                    CandidateLineage::Ranked(name, score) if *score > 0.0 =>
                                        Some(format!("{} ({})", name, score)),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let completions = candidates.completions();
                match self.expect.check(&completions.items) {
                    Ok(()) => {}
                    Err(e) => {
                        panic!(
                            "\n{}\nCompletion Test Failed\n{}\n\n\
                             Query:\n  {}\n\
                             Cursor position: {}\n\
                             Dialect: {}\n\
                             Error: {}\n\n\
                             Expected:\n{}\n\
                             Actual completions ({} total):\n{}\n\
                             Lineage:\n{}\n\
                             {}\n",
                            "=".repeat(80),
                            "=".repeat(80),
                            sql,
                            cursor,
                            spec.name,
                            e,
                            self.expect.format_expected(),
                            completions.items.len(),
                            fmt_list(&completions.items),
                            lineage_fmt,
                            "=".repeat(80)
                        )
                    }
                };
                results.extend(completions.items);
            }
        }
        results
    }
}

#[derive(Default)]
pub struct CompleterSet<'a> {
    items: Vec<Box<dyn Completer<'a> + 'a>>,
}

impl<'a> CompleterSet<'a> {
    pub fn add<P: Completer<'a> + Default + 'a>(mut self) -> Self {
        self.items.push(Box::new(P::default()));
        self
    }
}

impl<'a> Completer<'a> for CompleterSet<'a> {
    fn complete(&mut self, ctx: &mut Context<'a>, candidates: &mut CandidateSet<'a>) {
        for item in self.items.iter_mut() {
            if item.should_complete(ctx) {
                item.complete(ctx, candidates);
            }
        }
    }

    #[cfg(test)]
    fn debug_scores(&self) -> Option<std::collections::HashMap<String, Vec<(String, f32, f32)>>> {
        // Collect debug scores from all completers that have them
        let mut all_scores = std::collections::HashMap::new();
        for item in &self.items {
            if let Some(scores) = item.debug_scores() {
                for (label, ranker_scores) in scores {
                    all_scores
                        .entry(label)
                        .or_insert_with(Vec::new)
                        .extend(ranker_scores);
                }
            }
        }
        if all_scores.is_empty() {
            None
        } else {
            Some(all_scores)
        }
    }
}

/// Macro for testing SQL completion functionality.
///
/// This macro provides a declarative way to set up and run completion tests.
/// It accepts SQL queries with cursor positions (marked by caret `^`) and
/// expected completion results.
///
/// # Example
/// ```ignore
/// test_complete! {
///     "SELECT ^ FROM users" => {
///         contains: ["*", "id", "name"],
///         specs: [ansi::SPEC],
///         schemas: [users_schema()]
///     }
/// }
/// ```
///
/// # Supported Keys
/// - `contains`: Verifies all specified completions are present (order independent)
/// - `starts`: Verifies the first N completions match exactly
/// - `in_order`: Verifies completions appear in specific relative order
/// - `empty`: Verifies no completions are returned
/// - `specs`: Specifies dialect specifications to test against
/// - `completers`: Specifies completion providers to use
/// - `schemas`: Specifies database schemas for the test
#[macro_export]
macro_rules! test_complete {
    (
        $($sql:expr),+ $(,)? => {
            $(
                $k:ident $(: $vals:tt)?
            ),* $(,)?
        }
    ) => {{
        let mut exp = $crate::test_utils::Expect::new();
        let mut scn = $crate::test_utils::TestScenario::new();
        let mut completers = $crate::test_utils::CompleterSet::default();

        // SQLs + caret
        $(
            let (sql, cursor) = $crate::test_utils::get_caret_cursor($sql);
            if cursor > sql.len() {
                panic!(
                    "Cursor position {} exceeds SQL length {} in query: {}",
                    cursor,
                    sql.len(),
                    sql
                );
            }
            scn = scn.value((sql.to_string(), cursor));
        )+

        // config entries
        $(
            $crate::__push_matcher!($k $(: $vals)? => (exp, scn, completers));
        )*

        scn = scn.expect(exp);
        scn.run(completers)
    }};

    // no SQLs
    () => {
        compile_error!("test_complete! macro requires at least one SQL query");
    };

    // missing config after =>
    ($($sql:expr),+ $(,)? =>) => {
        compile_error!(
            "test_complete! macro requires configuration after '=>' (e.g., contains: [...])"
        );
    };
}

/// Internal helper macro for `test_complete!`.
///
/// This macro handles different matcher types (starts, contains, in_order, empty)
/// and configuration options (specs, completers, schemas) for the test scenario.
/// It is not intended to be used directly; use `test_complete!` instead.
#[macro_export]
macro_rules! __push_matcher {
    // starts: ["value", ...]
    (starts : [$($vals:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $exp = $exp.and($crate::test_utils::Starts(vec![$($vals.into()),*]));
    };

    // contains: ["value", ...]
    (contains : [$($vals:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $exp = $exp.and($crate::test_utils::Contains(vec![$($vals.into()),*]));
    };

    // in_order: ["value", ...]
    (in_order : [$($vals:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $exp = $exp.and($crate::test_utils::InOrder(vec![$($vals.into()),*]));
    };

    // empty: (no value) – flag syntax
    (none_of => ($exp:ident, $scn:ident, $comps:ident)) => {
        $exp = $exp.and($crate::test_utils::Empty(None));
    };

    // empty: [] – explicit empty list
    (none_of : [$($vals:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $exp = $exp.and($crate::test_utils::Empty(Some(vec![$($vals.into()),*])));
    };


    // specs: [...]
    (specs : [$($specs:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $scn = $scn.specs(vec![$($specs.clone()),*]);
    };

    // completers: [...]
    (completers : [$($completer:ty),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $( $comps = $comps.add::<$completer>(); )*
    };

    // schemas: [...]
    (schemas : [$($schemas:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $scn = $scn.schemas(vec![$($schemas),*]);
    };

    // Unknown key – keep the good error message
    ($unknown:ident $(: $($rest:tt)*)? => ($exp:ident, $scn:ident, $comps:ident)) => {
        compile_error!(
            concat!(
                "Unknown configuration key '",
                stringify!($unknown),
                "'. Supported keys are: contains, starts, in_order, empty, specs, completers, schemas"
            )
        )
    };
}
