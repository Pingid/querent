use std::collections::HashSet;

use crate::complete::{ColumnCompletion, CompletionBuilder, CompletionKind, PossibleCompletion};
use crate::schema;

use super::super::context::{ClauseKind, Context, Location, ResolvedColumnSource, ScopeResolve};

pub fn complete(ctx: &Context<'_>, builder: &mut CompletionBuilder) {
    if !supports(ctx) {
        return;
    }

    let available = ctx.scope.resolve_available_columns(ctx.schema);
    let projected = ctx.scope.resolve_projected_columns(ctx.schema);

    if available.is_empty() {
        // If we have already declared columns then only show columns from their tables
        let used_tables = projected
            .iter()
            .filter_map(|c| match c.source {
                ResolvedColumnSource::Schema(c) => Some(c.table_name.clone()),
                _ => None,
            })
            .collect::<HashSet<_>>();

        ctx.schema
            .get_columns()
            .iter()
            .filter(|x| used_tables.len() == 0 || used_tables.contains(&x.table_name))
            .map(|c| {
                let score = match ctx.scope.projected.iter().any(|x| x.name == c.column_name) {
                    true => -10,
                    false => 0,
                };
                make_completion(c.column_name.clone(), None, Some(c.clone()), score)
            })
            .for_each(|x| builder.add(x));
        return;
    }
    // println!("{:#?}", ctx.scope.resolve_available_columns(ctx.schema));
    // ctx.schema
    //     .get_columns()
    //     .iter()
    //     .map(|c| Col::from(c.clone()))
    //     .filter_map(|x| complete_qualifier(ctx, x))
    //     // .filter_map(|x| narrow_table_from_projected(ctx, x))
    //     .for_each(|x| {
    //         builder.add(make_completion(
    //             x.column_name.clone(),
    //             Some(x.table_name.clone()),
    //             None,
    //         ))
    //     });
}

fn supports(ctx: &Context<'_>) -> bool {
    match ctx.clause {
        ClauseKind::Select => is_select_position(ctx),
        _ => false,
    }
}

fn is_select_position(ctx: &Context<'_>) -> bool {
    match &ctx.cursor.location {
        Location::Space(inner) => {
            matches!(**inner, Location::Keyword | Location::Comma | Location::Dot)
        }
        Location::Dot => true,
        _ => false,
    }
}

// Common commit characters for column completions
const COMMIT_CHARS: [char; 4] = [',', ')', ' ', '\n'];

fn make_completion(
    name: String,
    source: Option<String>,
    column: Option<schema::Column>,
    score: i8,
) -> PossibleCompletion {
    PossibleCompletion {
        label: name.clone(),
        insert_text: name.clone(),
        filter_text: Some(name),
        kind: CompletionKind::Column(ColumnCompletion {
            qualifier: source,
            column,
        }),
        commit_characters: COMMIT_CHARS.into(),
        score,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        complete::Completions,
        test_util::{CompletionTest, CompletionTestExt},
    };

    use super::*;

    // #[test]
    // fn test_complete_qualifier() {
    //     let t = case("SELECT users.^");
    //     t.assert_col(["email", "id", "name"]);
    // }

    #[test]
    fn narrow_table_from_projected() {
        let t = case("SELECT ^ FROM (SELECT name as n FROM users) t");
        t.assert_col(["name", "email"]);
    }

    fn case(input: &str) -> Completions {
        CompletionTest::from_input(input)
            .with_users_posts()
            .run_with(complete)
    }
}
