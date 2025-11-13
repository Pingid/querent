use crate::complete::Completer;
use crate::complete::candidate::CandidateBuilder;
use crate::complete::candidate::CandidateSet;
use crate::complete::context::Context;

#[derive(Debug, Default)]
pub struct FunctionProvider;
impl Completer for FunctionProvider {
    fn complete<'a>(&mut self, ctx: &mut Context<'a>, b: &mut CandidateSet<'a>) {
        for func in ctx.functions() {
            b.push(
                CandidateBuilder::function(
                    func.function_name(),
                    func.return_type().data_type(),
                    func.parameter_types(),
                )
                .build(),
            );
        }
    }
}
