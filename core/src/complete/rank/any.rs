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
        #[derive(Debug)]
        pub enum AnyRanker {
            $( $Variant($Ty), )*
        }

        pub enum AnyRankerState<'ctx> {
            $( $Variant(<$Ty as Ranker>::State<'ctx>), )*
        }

        impl Ranker for AnyRanker {
            type State<'ctx> = AnyRankerState<'ctx>;
            fn init_state<'ctx>(&mut self, ctx: &Context<'ctx>) -> Self::State<'ctx> {
                match self {
                    $( AnyRanker::$Variant(r) => AnyRankerState::$Variant(r.init_state(ctx)), )*
                }
            }
            fn score<'ctx>(
                &self, cand: &Candidate<'ctx>, state: &mut Self::State<'ctx>, ctx: &Context<'ctx>,
            ) -> f32 {
                match (self, state) {
                    $( ( AnyRanker::$Variant(r), AnyRankerState::$Variant(s) ) => {
                        r.score(cand, s, ctx)
                    } )*
                    _ => 0.0,
                }
            }
        }

        // impl std::fmt::Debug for AnyRanker {
        //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //         match self {
        //             $( AnyRanker::$Variant(r) => write!(f, "{:?}", r), )*
        //         }
        //     }
        // }

        $(
            impl From<$Ty> for AnyRanker {
                fn from(r: $Ty) -> Self { AnyRanker::$Variant(r) }
            }

            // // cheap Debug for leaf rankers if they don't have one
            // impl std::fmt::Debug for $Ty {
            //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            //         write!(f, "{}", stringify!($Ty))
            //     }
            // }
        )*
    }
}

any_ranker_impls!(
    ColumnSource(ColumnSourceRank),
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
