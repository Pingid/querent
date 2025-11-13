use crate::{
    complete::{Completer, context::Context, types::Completion},
    dialect, schema,
    test_utils::CompletionSetMatch,
};

pub enum ScenarioPart {
    Sql(String, usize),
    Schema(schema::Cache),
    Spec(dialect::DialectSpec),
    Completer(Box<dyn Completer<'static>>),
    Match(CompletionSetMatch),
    And(Vec<ScenarioPart>),
}

impl ScenarioPart {
    pub fn and(self, other: Self) -> Self {
        Self::And(vec![self, other])
    }
    #[track_caller]
    pub fn run(self) -> Vec<Completion> {
        let Self::And(mut parts) = self else {
            panic!("ScenarioPart::run called on a non-and part");
        };
        let mut statements: Vec<(String, usize)> = vec![];
        let mut schema = schema::Cache::default();
        let mut specs = vec![];
        let mut completers = vec![];
        let mut matches = vec![];

        while let Some(part) = parts.pop() {
            match part {
                ScenarioPart::Sql(sql, cursor) => statements.push((sql, cursor)),
                ScenarioPart::Schema(s) => schema = schema.combine(s),
                ScenarioPart::Spec(spec) => specs.push(spec),
                ScenarioPart::Completer(c) => completers.push(c),
                ScenarioPart::Match(m) => matches.push(m),
                ScenarioPart::And(p) => parts.extend(p),
            }
        }

        let mut results = vec![];

        for spec in specs {
            for (sql, cursor) in statements.iter() {
                let mut ctx = Context::build(&spec, &schema, &sql, *cursor).unwrap();
                //     // let mut candidates = CandidateSet::default();
                //     // for completer in completers.iter_mut() {
                //         // if completer.should_complete(&ctx) {
                //         //     // completer.complete(&mut ctx, &mut candidates);
                //         // }
                //     }
            }
        }

        results
    }
}
