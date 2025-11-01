use crate::complete::completion::Candidate;
use crate::complete::context::Context;
use crate::complete::rank::Ranker;
use crate::complete::rank::basic::*;
use crate::complete::rank::column_qualifier::*;
use crate::complete::rank::column_source::*;

macro_rules! any_ranker_impls {
    ( $( $Variant:ident ( $Ty:ty ) ),+ $(,)? ) => {
        pub enum AnyRanker<'a> {
            $( $Variant($Ty), )*
        }

        impl<'a> Ranker<'a> for AnyRanker<'a> {
            fn prepare(&mut self, ctx: &Context<'a>) {
                match self {
                    $( AnyRanker::$Variant(r) => r.prepare(ctx), )*
                }
            }
            fn score(&self, ctx: &Context<'a>, cand: &Candidate<'a>) -> f32 {
                match self {
                    $( AnyRanker::$Variant(r) => Ranker::score(r, ctx, cand), )*
                }
            }
        }

        $(
            impl<'a> From<$Ty> for AnyRanker<'a> {
                fn from(r: $Ty) -> Self { AnyRanker::$Variant(r) }
            }
        )*
    }
}

any_ranker_impls!(
    ColumnSource(ColumnSourceRank<'a>),
    ColumnQualified(ColumnQualifiedRank),
    KindMatch(KindMatchRank),
    TypeCompat(TypeCompatRank),
);
