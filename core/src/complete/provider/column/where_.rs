use super::helper::get_scope_available_columns;
use crate::complete::completion::Completion;
use crate::complete::completion::CompletionBuilder;
use crate::complete::completion::CompletionKind;
use crate::complete::context::ClauseKind;
use crate::complete::context::Context;
use crate::complete::context::Location;
use crate::complete::provider::column::helper::AvailableColumn;
use crate::complete::provider::column::helper::get_qualified_name;
use crate::lex::Keyword;
use crate::lex::OpTag;

pub fn complete(ctx: &mut Context<'_>, builder: &mut CompletionBuilder) {
    if !should_complete(ctx) {
        return;
    }

    // Add relation columns with qualified name
    let available = get_scope_available_columns(ctx);
    for col in available {
        let label = match ctx.cursor.qualifier.is_some() {
            true => col.name().to_string(),
            false => get_qualified_name(&col).unwrap_or_else(|| col.name().to_string()),
        };
        builder.add(
            Completion::new(
                CompletionKind::Column,
                label,
                ctx.cursor.replace,
                None,
                Some(col.detail()),
            ),
            col.score(),
        );
    }

    if ctx.cursor.qualifier.is_some() {
        return;
    }

    // Rank columns that appear in the SELECT list higher
    let projected = ctx.scope.projected();

    for p in projected {
        let col = AvailableColumn::from(p.clone());
        builder.add(
            Completion::new(
                CompletionKind::Column,
                col.name().to_string(),
                ctx.cursor.replace,
                None,
                Some(col.detail()),
            ),
            20,
        );
    }
}

fn should_complete(ctx: &Context<'_>) -> bool {
    match ctx.clause {
        ClauseKind::Where => match &ctx.cursor.location {
            Location::Space(inner) => matches!(
                **inner,
                Location::Keyword(Keyword::Where)
                    | Location::Operator(OpTag::And)
                    | Location::Operator(OpTag::Or)
            ),
            Location::Dot => true,
            Location::Ident => true,
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::CompletionTest;
    use crate::test_util::CompletionTestResult;

    #[test]
    fn skips_at_inappropriate_locations() {
        case("SELECT a FROM b WHERE a ^").assert_empty();
        case("SELECT a FROM b WHERE a AND^").assert_empty();
        case("SELECT * FROM users WHERE name =^").assert_empty();
        case("SELECT * FROM users WHERE name = ^").assert_empty();
        case("SELECT * FROM users WHERE name = 'John'^").assert_empty();
        case("SELECT * FROM users WHERE name = 'John' AND^").assert_empty();
        case("SELECT * FROM users WHERE name = 'John' AND ^").assert_not_empty();
    }

    #[test]
    fn ranks_projected_columns_higher() {
        let t = case("SELECT * FROM users WHERE ^");
        t.assert_labels(["email", "id", "name"]);
    }

    #[test]
    fn completes_aliased_columns() {
        let t = case("SELECT email as email_alias FROM users WHERE ^");
        t.assert_labels(["email_alias", "users.email", "users.id"]);
    }

    #[test]
    fn completes_columns_from_cte() {
        let t = case("WITH cte as (SELECT * FROM users) SELECT * FROM cte WHERE ^");
        t.assert_labels(["email", "id", "name", "cte.email", "cte.id", "cte.name"]);
    }

    #[test]
    fn completes_after_qualifier_dot() {
        let t = case("WITH cte as (SELECT * FROM users) SELECT * FROM cte WHERE cte.^");
        t.assert_labels(["email", "id", "name"]);
        t.assert_missing_labels(["cte.email", "cte.id", "cte.name"]);
    }

    fn case(input: &str) -> CompletionTestResult {
        CompletionTest::from_input(input)
            .with_users_posts()
            .run_with(complete)
    }
}
