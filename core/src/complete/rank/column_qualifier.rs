use std::collections::HashSet;

use crate::complete::candidate::Candidate;
use crate::complete::candidate::ColumnCandidate;
use crate::complete::context::Context;
use crate::complete::rank::ColumnRanker;

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
pub struct ColumnQualifiedRank {
    prioritize_unqualified: bool,
}

impl ColumnQualifiedRank {
    pub const fn new() -> Self {
        Self {
            prioritize_unqualified: true,
        }
    }
}

impl<'a> ColumnRanker<'a> for ColumnQualifiedRank {
    fn prepare(&mut self, ctx: &Context<'a>) {
        self.prioritize_unqualified = true;

        let mut names = HashSet::new();
        for p in ctx.scope().available() {
            if !names.insert(p.label.name()) {
                self.prioritize_unqualified = false;
                return;
            }
        }
    }
    fn score_column(&self, _: &Context<'_>, _: &Candidate, col: &ColumnCandidate<'_>) -> f32 {
        match (
            self.prioritize_unqualified,
            col.label.parent,
            col.label.schema,
        ) {
            // Unqualified
            (true, None, None) => 1.0,
            (true, Some(_), _) => 0.8,
            (true, _, Some(_)) => 0.6,
            // Qualified
            (false, None, None) => 0.0,
            (false, Some(_), _) => 1.0,
            (false, _, Some(_)) => 0.8,
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
            completers: [DefaultProviders, ColumnQualifiedRank::new()],
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
            completers: [DefaultProviders, ColumnQualifiedRank::new()],
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
            completers: [DefaultProviders, ColumnQualifiedRank::new()],
            schemas: [users_schema(), posts_schema()],
            in_order: ["users.name", "name"],
        });
    }
}
