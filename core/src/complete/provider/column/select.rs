use std::collections::HashMap;

use crate::complete::provider::column::helper::get_qualified_name;
use crate::lex::{Keyword, TokenKind};

use crate::complete::context::{ClauseKind, Context, Location, ResolvedColumn, ScopeResolve};
use crate::complete::{Completion, CompletionBuilder, CompletionKind};

use super::helper::get_scope_available_columns;

pub fn complete(ctx: &Context<'_>, builder: &mut CompletionBuilder) {
    if !should_complete(ctx) {
        return;
    }

    let mut cols = get_scope_available_columns(ctx);

    let projected = ctx.scope.resolve_projected_columns(ctx.schema);

    // Count the number of projected columns for each source name
    let mut source_counts: HashMap<String, i8> = HashMap::new();
    for col in projected.iter() {
        if let Some(source_name) = col.source_name() {
            *source_counts.entry(source_name.to_string()).or_insert(0i8) += 1;
        }
    }

    // Update scores for columns based on projected columns.
    cols.iter_mut().for_each(|col| {
        // If the column source name has more than one projected column, decrease score
        if let Some(source_name) = col.source_name() {
            if let Some(count) = source_counts.get(&source_name) {
                col.update_score(10 * count);
            }
        }

        // If the column name matches a projected column name decrease score
        if projected.iter().any(|p| p.name == *col.name()) {
            col.update_score(-1);
        }
    });

    // Check if there are columns with the same name but different source
    // This is used to score qualified columns higher if there are conflicts
    let has_columns_with_same_name_different_source = cols.iter().any(|col| {
        cols.iter()
            .any(|c| c.name() == col.name() && c.source_name() != col.source_name())
    });

    cols.into_iter().for_each(|col| {
        let col_name = col.name().to_string();
        builder.add(
            Completion::new(
                CompletionKind::Column,
                col_name.clone(),
                ctx.cursor.replace,
                None,
                Some(col.detail()),
            ),
            col.score(),
        );

        if let Some(label) = get_qualified_name(&col)
            && ctx.cursor.qualifier.is_none()
        {
            let score = get_score_qualified_column(
                &projected,
                has_columns_with_same_name_different_source,
                col.source_name(),
            );
            builder.add(
                Completion::new(
                    CompletionKind::Column,
                    label,
                    ctx.cursor.replace,
                    None,
                    Some(col.detail()),
                ),
                col.score() + score,
            );
        }
    });
}

fn should_complete(ctx: &Context<'_>) -> bool {
    match ctx.clause {
        ClauseKind::Select => match &ctx.cursor.location {
            Location::Space(inner) => matches!(
                **inner,
                Location::Comma | Location::Keyword(Keyword::Select)
            ),
            Location::Dot => true,
            Location::Ident
                if ctx
                    .cursor
                    .preceding_matches([TokenKind::Comma, TokenKind::Identifier]) =>
            {
                true
            }
            _ => false,
        },
        _ => false,
    }
}

fn get_score_qualified_column(
    projected: &Vec<ResolvedColumn<'_, '_>>,
    has_column_conflicts: bool,
    table_name: Option<String>,
) -> i8 {
    let Some(table_name) = table_name else {
        return 0;
    };
    // Check if there is a projected column from the same table using the qualified syntax
    let found_projection_using_qualified = projected
        .iter()
        .rev()
        .find(|p| p.matches_source_name(&table_name))
        .map_or(false, |p| !p.qualifier.is_empty());
    // If a projected column from the same table using the qualified syntax is found, score it higher
    match found_projection_using_qualified {
        true => 10,
        false => match projected.is_empty() && has_column_conflicts {
            true => 10,
            false => -10,
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::{CompletionTest, CompletionTestResult};

    use super::*;

    #[test]
    fn skips_at_inappropriate_locations() {
        case("SELECT a ^").assert_empty();
        case("SELECT a as^").assert_empty();
        case("SELECT a as b^").assert_empty();
        case("SELECT a as b ^").assert_empty();
    }

    #[test]
    fn completes_all_columns_when_empty() {
        let t = case("SELECT ^");
        t.assert_labels([
            "posts.content",
            "posts.id",
            "posts.title",
            "users.email",
            "users.id",
            "users.name",
        ]);
    }

    #[test]
    fn narrows_results_by_projected_columns() {
        let t = case("SELECT email, ^");
        t.assert_labels(["id", "name"]);
    }

    #[test]
    fn narrows_results_by_qualified_columns() {
        let t = case("SELECT users.email, ^");
        t.assert_labels(["users.id", "users.name"]);
    }

    #[test]
    fn completes_after_qualifier_dot() {
        let t = case("SELECT users.^");
        t.assert_labels(["email", "id", "name"]);
    }

    #[test]
    fn completes_aliased_columns_from_subquery() {
        let t = case("SELECT ^ FROM (SELECT email as u_email FROM users) u");
        t.assert_labels(["u_email"]);
    }

    #[test]
    fn completes_qualified_columns_from_subquery() {
        let t = case("SELECT u.email, ^ FROM (SELECT 10 as age, * FROM users) u");
        t.assert_labels(["u.age", "u.id", "u.name"]);
    }

    #[test]
    fn completes_after_qualifier_dot_from_multiple_subqueries() {
        let t = case("SELECT u.email, p.^ FROM (SELECT * FROM users) u, (SELECT * FROM posts) p;");
        t.assert_labels(["content", "id", "title"]);
    }

    #[test]
    fn completes_columns_from_cte() {
        let t = case("WITH u AS (SELECT id, email FROM users) SELECT ^ FROM u");
        t.assert_labels(["email", "id"]);
    }

    fn case(input: &str) -> CompletionTestResult {
        CompletionTest::from_input(input)
            .with_users_posts()
            .run_with(complete)
    }
}
