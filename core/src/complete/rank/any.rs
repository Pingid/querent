use crate::complete::candidate::Candidate;
use crate::complete::context::Context;
use crate::complete::rank::Ranker;
use crate::complete::rank::basic::*;
use crate::complete::rank::column_qualifier::*;
use crate::complete::rank::column_source::*;
use crate::complete::rank::keyword::KeywordMatchRank;
use crate::complete::rank::table_qualifier::*;

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

        impl<'a> std::fmt::Debug for AnyRanker<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( AnyRanker::$Variant(r) => write!(f, "{:?}", r), )*
                }
            }
        }

        $(
            impl<'a> From<$Ty> for AnyRanker<'a> {
                fn from(r: $Ty) -> Self { AnyRanker::$Variant(r) }
            }

            impl<'a> std::fmt::Debug for $Ty {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{:?}", stringify!($Ty))
                }
            }
        )*
    }
}

any_ranker_impls!(
    ColumnSource(ColumnSourceRank<'a>),
    ColumnQualified(ColumnQualifiedRank),
    TableQualified(TableQualifiedRank),
    KindMatch(KindMatchRank),
    KeywordMatch(KeywordMatchRank),
    TypeCompat(TypeCompatRank),
    Ignore(IgnoreRank),
    ExactMatch(ExactMatchRanker),
    PrefixMatch(PrefixMatchRanker),
    FuzzyMatch(FuzzyMatchRanker),
);
