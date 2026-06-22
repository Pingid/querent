use crate::complete::candidate::Candidate;
use crate::complete::candidate::CandidateKind;
use crate::complete::context::ClauseKind;
use crate::complete::context::Context;
use crate::complete::rank::Ranker;

/// Ranks table completions based on qualification level.
/// Prioritizes unqualified names (e.g., "users") over qualified names (e.g., "public.users")
/// when there's no ambiguity.
#[derive(Debug, Default)]
pub struct TableQualifiedRankState {
    prioritize_unqualified: bool,
}

#[derive(Debug, Default)]
pub struct TableQualifiedRank;
impl Ranker for TableQualifiedRank {
    type State<'ctx> = TableQualifiedRankState;
    fn init_state<'ctx>(&mut self, _ctx: &Context<'ctx>) -> Self::State<'ctx> {
        TableQualifiedRankState {
            prioritize_unqualified: true,
        }
    }
    fn score<'ctx>(
        &self, cand: &Candidate<'ctx>, state: &mut Self::State<'ctx>, _ctx: &Context<'ctx>,
    ) -> f32 {
        match &cand.kind {
            CandidateKind::Table(table) => {
                if state.prioritize_unqualified {
                    // Prioritize unqualified names
                    if table.label.schema.is_none() {
                        return 2.0; // Unqualified (e.g., users)
                    } else {
                        return 1.0; // Schema-qualified (e.g., public.users)
                    }
                }
                1.0
            }
            _ => 0.0,
        }
    }
}

/// Boost FROM-clause tables that contain the columns already projected in the
/// SELECT list, so `SELECT email FROM ^` surfaces `users` ahead of `posts`.
#[derive(Debug, Default)]
pub struct TableColumnMatchRank;

#[derive(Debug, Default)]
pub struct TableColumnMatchRankState<'a> {
    projected: Vec<&'a str>,
}

impl Ranker for TableColumnMatchRank {
    type State<'ctx> = TableColumnMatchRankState<'ctx>;
    fn init_state<'ctx>(&mut self, ctx: &Context<'ctx>) -> Self::State<'ctx> {
        let projected = match ctx.clause().kind {
            ClauseKind::From => ctx
                .scope()
                .projected()
                .iter()
                .map(|p| p.label.name())
                .filter(|name| *name != "*")
                .collect(),
            _ => Vec::new(),
        };
        TableColumnMatchRankState { projected }
    }
    fn score<'ctx>(
        &self, cand: &Candidate<'ctx>, state: &mut Self::State<'ctx>, ctx: &Context<'ctx>,
    ) -> f32 {
        let CandidateKind::Table(table) = &cand.kind else {
            return 0.0;
        };
        if state.projected.is_empty() {
            return 0.0;
        }
        let name = table.ident.name();
        let schema = table.ident.schema();
        let columns = ctx.schema().get_columns();
        let contained = state
            .projected
            .iter()
            .filter(|col| {
                columns.iter().any(|c| {
                    c.column_name == **col
                        && c.table_name.as_deref() == Some(name)
                        && schema.is_none_or(|s| c.schema_name.as_deref() == Some(s))
                })
            })
            .count();
        // Fraction of projected columns the table provides.
        contained as f32 / state.projected.len() as f32
    }
}
