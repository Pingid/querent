use std::collections::HashSet;

use crate::complete::candidate::{Candidate, CandidateKind};
use crate::complete::context::Context;
use crate::complete::rank::Ranker;

/// ### Prioritize unqualified names when:
/// - There's only one table in the FROM clause
/// - The column name is unique across all tables in the query
/// - The user is in a simple SELECT/WHERE context with no ambiguity
/// - The user has already started typing an unqualified name
/// ### Prioritize qualified names when:
/// - Multiple tables are joined
/// - The column exists in multiple tables (ambiguous)
/// - The user has started typing a table name or alias followed by a dot
/// - The query is complex with subqueries or CTEs.
pub struct ColumnQualifiedRankState {
    prioritize_unqualified: bool,
    has_table_qualified_projections: bool,
}

#[derive(Debug, Default)]
pub struct ColumnQualifiedRank;
impl Ranker for ColumnQualifiedRank {
    type State<'ctx> = ColumnQualifiedRankState;
    fn init_state<'ctx>(&mut self, ctx: &Context<'ctx>) -> Self::State<'ctx> {
        let mut state = ColumnQualifiedRankState {
            prioritize_unqualified: true,
            has_table_qualified_projections: false,
        };

        // Check if we have any bindings (FROM clause tables)
        // If there are no bindings, we're in a query without FROM clause
        // In such cases, don't try to detect qualification patterns from projections
        // since they'll be resolved from schema with table names added
        let has_bindings = !ctx.scope().bindings.is_empty();

        if has_bindings {
            // Check if any existing projections use table-qualified names
            // We only check for table qualification (parent) since schema/database
            // might be added during resolution even for originally unqualified names
            let projections = ctx.scope().projected();

            // If we have projections, check their qualification level
            if !projections.is_empty() {
                for p in projections {
                    if p.label.parent.is_some() {
                        // Found a table-qualified column in the existing projections
                        state.prioritize_unqualified = false;
                        state.has_table_qualified_projections = true;
                        return state;
                    }
                }
                // If we have projections but none are qualified, keep unqualified as priority
                // This handles cases like "SELECT name, ^ FROM users" where name is unqualified
                return state;
            }
        }

        // Check for ambiguous columns in available projections
        let mut names = HashSet::new();
        for p in ctx.scope().available() {
            if !names.insert(p.label.name()) {
                state.prioritize_unqualified = false;
                return state;
            }
        }
        state
    }
    fn score<'ctx>(
        &self, cand: &Candidate<'ctx>, state: &mut Self::State<'ctx>, _ctx: &Context<'ctx>,
    ) -> f32 {
        let CandidateKind::Column(col) = &cand.kind else {
            return 0.0;
        };
        // let So
        // If we detected table-qualified names in existing projections
        if state.has_table_qualified_projections {
            // Prioritize table-qualified columns
            if col.label.parent.is_some() && col.label.schema.is_none() {
                return 1.0; // Table-qualified only (e.g., users.name)
            } else if col.label.parent.is_none() && col.label.schema.is_none() {
                return 0.3; // Unqualified (e.g., name) - lower to ensure qualified appear first
            } else {
                return 0.4; // Schema-qualified or database-qualified
            }
        }

        // Original logic for when no existing pattern is detected
        match (
            state.prioritize_unqualified,
            col.label.parent,
            col.label.schema,
            col.label.database,
        ) {
            // When prioritizing unqualified names
            (true, None, None, _) => 1.0,       // Unqualified - highest
            (true, Some(_), None, None) => 0.7, // Table-qualified - still good
            (true, _, Some(_), None) => 0.5,    // Schema-qualified - lower
            (true, _, _, Some(_)) => 0.4,       // Database-qualified - lowest
            // When prioritizing qualified names
            (false, Some(_), None, None) => 1.0, // Table-qualified - highest
            (false, Some(_), Some(_), None) => 0.8, // Schema.table-qualified
            (false, Some(_), _, Some(_)) => 0.6, // Database.schema.table-qualified
            (false, None, None, _) => 0.5,       // Unqualified - still usable
            (false, _, _, _) => 0.3,             // Other combinations
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complete::provider::DefaultProviders;
    use crate::test_complete;
    use crate::test_utils::posts_schema;
    use crate::test_utils::users_schema;

    #[test]
    fn ranks_unqualified_columns_higher() {
        // No tables in the FROM clause
        test_complete!("SELECT ^ " => {
            completers: [DefaultProviders, ColumnQualifiedRank],
            schemas: [users_schema(), posts_schema()],
            in_order: ["name", "title", "posts.title", "users.name"],
        });
        // Single CTE
        // test_complete!("WITH user_posts AS (SELECT * FROM users) SELECT ^ FROM user_posts" => {
        //     completers: [DefaultProviders, ColumnQualifiedRank::default()],
        //     schemas: [users_schema(), posts_schema()],
        //     in_order: ["email", "name", "users.email", "users.name"],
        // });
    }

    #[test]
    fn single_table_prioritizes_unqualified() {
        // When there's only one table, unqualified columns should rank higher
        test_complete!("SELECT ^ FROM users" => {
            completers: [DefaultProviders, ColumnQualifiedRank],
            schemas: [users_schema()],
            in_order: ["email", "id", "name", "users.email", "users.id", "users.name"],
        });

        // // Unqualified columns should rank higher in WHERE clause with single table
        // test_complete!("SELECT * FROM users WHERE ^ = 1" => {
        //     completers: [Providers::default(), rank],
        //     schemas: [users_schema()],
        //     in_order: ["id", "name", "email", "users.id", "users.name", "users.email"],
        // });
    }

    #[test]
    fn ambiguous_column_prioritizes_qualified() {
        // When column exists in multiple tables (like 'id'), qualified should rank higher
        test_complete!("SELECT ^ FROM users, posts" => {
            completers: [DefaultProviders, ColumnQualifiedRank],
            schemas: [users_schema(), posts_schema()],
            in_order: ["users.name", "name"],
        });
    }
}
