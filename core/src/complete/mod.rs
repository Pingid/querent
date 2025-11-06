use crate::complete::candidate::CandidateSet;
use crate::complete::context::Context;

pub mod candidate;
pub mod context;
pub mod engine;
pub mod provider;
pub mod rank;
pub mod types;

#[cfg(test)]
pub mod test_util;

pub trait Completer<'a> {
    fn complete(&mut self, ctx: &mut Context<'a>, builder: &mut CandidateSet<'a>);
    fn should_complete(&self, _ctx: &Context<'a>) -> bool {
        true
    }
}
