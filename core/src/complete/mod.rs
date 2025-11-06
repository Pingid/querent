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

pub trait Completer<'a> {
    fn complete(&mut self, ctx: &mut Context<'a>, builder: &mut CandidateSet<'a>);

    fn should_complete(&self, _ctx: &Context<'a>) -> bool {
        true
    }

    #[cfg(test)]
    /// Returns a map of candidate labels to ranker name, score, and weight.
    fn debug_scores(&self) -> Option<std::collections::HashMap<String, Vec<(String, f32, f32)>>> {
        None
    }
}

#[derive(Default)]
pub struct DefaultCompleter<'a> {
    ranker: DefaultRanker<'a>,
    providers: DefaultProviders,
}

impl<'a> Completer<'a> for DefaultCompleter<'a> {
    fn complete(&mut self, ctx: &mut Context<'a>, builder: &mut CandidateSet<'a>) {
        self.providers.complete(ctx, builder);
        self.ranker.complete(ctx, builder);
    }
    #[cfg(test)]
    fn debug_scores(&self) -> Option<std::collections::HashMap<String, Vec<(String, f32, f32)>>> {
        self.ranker.debug_scores()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::ansi;
    use crate::test_complete;

    #[test]
    fn keyword_completes_partial_at_start() {
        // Complete partial keyword at statement start
        test_complete!("SELE^" => {
            contains: ["SELECT"],
            specs: [ansi::SPEC],
            starts: ["SELECT"],
            completers: [DefaultCompleter],
        });
    }
}
