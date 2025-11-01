use crate::complete::Completer;
use crate::complete::completion::CandidateSet;
use crate::complete::context::Context;
use crate::complete::providers::column::ColumnProvider;

macro_rules! any_provider_impls {
    ($($n:ident),+ $(,)?) => {
        pub enum AnyProvider {
            $( $n($n), )*
        }
        impl<'a> Completer<'a> for AnyProvider {
            fn complete(&mut self, ctx: &mut Context<'a>, builder: &mut CandidateSet<'a>) {
                match self {
                    $( AnyProvider::$n(r) => r.complete(ctx, builder), )*
                }
            }
        }
        $(
            impl From<$n> for AnyProvider {
                fn from(r: $n) -> Self {
                    AnyProvider::$n(r)
                }
            }
        )*
    }
}

any_provider_impls!(ColumnProvider);
