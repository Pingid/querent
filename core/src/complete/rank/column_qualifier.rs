use crate::complete::completion::Candidate;
use crate::complete::completion::CandidateKind;
use crate::complete::completion::ColumnCandidate;
use crate::complete::context::Context;
use crate::complete::rank::ColumnRanker;
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
pub struct ColumnQualifiedRank;
impl<'a> ColumnRanker<'a> for ColumnQualifiedRank {
    fn score_column(&self, ctx: &Context<'_>, cand: &Candidate, col: &ColumnCandidate<'_>) -> f32 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complete::completion::CandidateSet;
    use crate::complete::providers::DefaultProviders;
    use crate::test_complete;
    use crate::test_utils::posts_schema;
    use crate::test_utils::users_schema;

    fn rank<'a>(ctx: &mut Context<'a>, b: &mut CandidateSet<'a>) {
        for cand in b.items.iter_mut() {
            cand.score = ColumnQualifiedRank.score(ctx, cand);
        }
    }

    #[test]
    fn ranks_unqualified_columns_higher() {
        test_complete!("SELECT ^ FROM users" => {
            completers: [DefaultProviders, ColumnQualifiedRank],
            schemas: [users_schema()],
            in_order: ["name", "users.name"],
        });
    }

    #[test]
    fn single_table_prioritizes_unqualified() {
        // When there's only one table, unqualified columns should rank higher
        test_complete!("SELECT ^ FROM users" => {
            completers: [DefaultProviders, ColumnQualifiedRank],
            schemas: [users_schema()],
            in_order: ["id", "name", "email", "users.id", "users.name", "users.email"],
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

    #[test]
    fn multi_table_join_prioritizes_qualified() {
        // With multiple tables, qualified columns should rank higher
        test_complete!("SELECT ^ FROM users u JOIN posts p ON u.id = p.user_id" => {
            completers: [DefaultProviders, ColumnQualifiedRank],
            schemas: [users_schema(), posts_schema()],
            in_order: ["u.name", "p.title", "name", "title"],
        });
    }

    #[test]
    fn cte_prioritizes_qualified() {
        // CTEs should prioritize qualified names
        test_complete!("WITH user_posts AS (SELECT * FROM users) SELECT ^ FROM user_posts up JOIN posts p" => {
            completers: [DefaultProviders, ColumnQualifiedRank],
            schemas: [users_schema(), posts_schema()],
            in_order: ["u.name", "p.title", "name", "title"],
        });
    }
}
