use crate::complete::Completer;
use crate::complete::candidate::CandidateSet;
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
                let completions = candidates.completions();
                match self.expect.check(&completions.items) {
                    Ok(()) => {}
                    Err(e) => panic!(" QUERY: {}\n  SPEC: {}\nFAILED: {}", sql, spec.name, e),
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
    pub fn add<P: Completer<'a> + 'a>(mut self, item: P) -> Self {
        self.items.push(Box::new(item));
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
#[macro_export]
macro_rules! test_complete {
    (
        $($sql:expr),* => {
        $($k:ident : [$($vals:expr),* $(,)?]),* $(,)?
      }
    ) => {{
        let mut exp = crate::test_utils::Expect::new();
        let mut scn = crate::test_utils::TestScenario::new();
        let mut completers = crate::test_utils::CompleterSet::default();

        $(
            let (sql, cursor) = crate::test_utils::get_caret_cursor($sql);
            scn = scn.value((sql.to_string(), cursor));
        )*

        $(
            crate::__push_matcher!($k : [$($vals),*] => (exp, scn, completers));
        )*
        scn = scn.expect(exp);
        scn.run(completers)
    }};
}

/// Internal helper macro for `test_complete!`.
///
/// This macro handles different matcher types (starts, contains, in_order, empty)
/// and configuration options (specs, completers, schemas) for the test scenario.
/// It is not intended to be used directly; use `test_complete!` instead.
#[macro_export]
macro_rules! __push_matcher {
    (starts : [$($vals:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $exp = $exp.and(crate::test_utils::Starts(vec![$($vals.into()),*]));
    };
    (contains : [$($vals:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $exp = $exp.and(crate::test_utils::Contains(vec![$($vals.into()),*]));
    };
    (in_order : [$($vals:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $exp = $exp.and(crate::test_utils::InOrder(vec![$($vals.into()),*]));
    };
    (empty : [$($vals:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $exp = $exp.and(crate::test_utils::Empty);
    };
    (specs : [$($specs:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $scn = $scn.specs(vec![$($specs.clone()),*]);
    };
    (completers : [$($completer:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $( $comps = $comps.add($completer); )*
    };
    (schemas : [$($schemas:expr),* $(,)?] => ($exp:ident, $scn:ident, $comps:ident)) => {
        $scn = $scn.schemas(vec![$($schemas),*]);
    };
}
