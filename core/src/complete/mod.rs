use crate::complete::candidate::CandidateSet;
use crate::complete::context::Context;
use crate::complete::provider::DefaultProviders;
use crate::complete::rank::DefaultRanker;

pub mod candidate;
pub mod context;
pub mod engine;
pub mod provider;
pub mod rank;
pub mod types;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_util;

pub trait Completer: std::fmt::Debug {
    fn complete<'a>(&mut self, ctx: &mut Context<'a>, builder: &mut CandidateSet<'a>);

    fn should_complete<'a>(&self, _ctx: &Context<'a>) -> bool {
        true
    }
}

#[derive(Debug, Default)]
pub struct DefaultCompleter {
    ranker: DefaultRanker,
    providers: DefaultProviders,
}

impl Completer for DefaultCompleter {
    fn complete<'a>(&mut self, ctx: &mut Context<'a>, builder: &mut CandidateSet<'a>) {
        self.providers.complete(ctx, builder);
        self.ranker.complete(ctx, builder);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::ansi;
    use crate::test_utils::ScenarioComp;

    #[test]
    fn keyword_completes_partial_at_start() {
        // Complete partial keyword at statement start
        ScenarioComp::default()
            .completer(DefaultCompleter::default())
            .spec(ansi::SPEC.clone())
            .query("SELE^")
            .starts(["SELECT"])
            .run();
    }
}
