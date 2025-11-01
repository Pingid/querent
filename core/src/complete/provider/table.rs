use crate::complete::completion::Completion;
use crate::complete::completion::CompletionBuilder;
use crate::complete::completion::CompletionKind;
use crate::complete::context::BindingKind;
use crate::complete::context::ClauseKind;
use crate::complete::context::Context;
use crate::complete::context::Location;
use crate::lex::Keyword;
use crate::lex::TokenKind;
use crate::schema;

pub fn complete(ctx: &mut Context<'_>, builder: &mut CompletionBuilder) {
    if !should_complete(ctx) {
        return;
    }

    let mut tables = Vec::new();

    // Add all tables from the schema
    for table in ctx.schema().get_tables() {
        tables.push(AvailableTable {
            name: table.table_name.clone(),
            score: 0,
            kind: AvailableTableKind::Table(table.clone()),
        });
    }

    // Add all CTEs
    for cte in ctx.scope.ctes() {
        tables.push(AvailableTable {
            name: cte.to_string(),
            score: 0,
            kind: AvailableTableKind::Cte,
        });
    }

    // Rank tables used in the SELECT list higher
    let projected = ctx.scope.projected();

    for t in &mut tables {
        if projected.iter().any(|p| match &p.source_alias {
            Some(alias) => alias == &t.name,
            None => p.source.table_name() == Some(&t.name),
        }) {
            t.score += 20;
        }
    }

    // Rank tables already used in the FROM clause lower
    for t in &mut tables {
        if ctx.scope.bindings().any(|r| match &r.kind {
            BindingKind::Base(path) => path.0.contains(&t.name.as_str()),
            _ => false,
        }) {
            t.score -= 30;
        }
    }

    for t in tables {
        let detail = detail(&t);
        // println!("table: {:#?}, score: {}", &t, t.score);
        builder.add(
            Completion::new(
                CompletionKind::Table,
                t.name,
                ctx.cursor.replace,
                None,
                Some(detail),
            ),
            t.score,
            None,
        );
    }
}

fn should_complete(ctx: &Context<'_>) -> bool {
    match ctx.clause.kind {
        ClauseKind::From => match &ctx.cursor.location {
            Location::Space(inner) => {
                matches!(**inner, Location::Keyword(Keyword::From) | Location::Comma)
            }
            Location::Ident
                if ctx.cursor.preceding_matches([
                    TokenKind::Keyword(Keyword::From),
                    TokenKind::Identifier,
                ]) =>
            {
                true
            }
            _ => false,
        },
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct AvailableTable {
    name: String,
    score: i8,
    kind: AvailableTableKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AvailableTableKind {
    Table(schema::Table),
    Cte,
}

fn detail(table: &AvailableTable) -> String {
    match &table.kind {
        AvailableTableKind::Table(t) => {
            let qualified = match &t.schema_name {
                Some(schema) => format!("{}.{}", schema, &t.table_name),
                None => t.table_name.clone(),
            };
            let tp = match table.kind {
                AvailableTableKind::Table(_) => "table",
                AvailableTableKind::Cte => "cte",
            };
            format!("{} ({})", qualified, tp)
        }
        AvailableTableKind::Cte => format!("{} (cte)", table.name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::CompletionTest;
    use crate::test_util::CompletionTestResult;

    #[test]
    fn completes_at_appropriate_locations() {
        case("SELECT email FROM^").assert_empty();
        case("SELECT email FROM ^").assert_not_empty();
        case("SELECT email FROM u^").assert_not_empty();
        case("SELECT email FROM user ^").assert_empty();
        case("SELECT email FROM user, ^").assert_not_empty();

        case("SELECT email FROM user t ^").assert_empty();
        case("SELECT email FROM user as t ^").assert_empty();
        case("SELECT email FROM user t, ^").assert_not_empty();
        case("SELECT email FROM user as t, ^").assert_not_empty();
    }

    #[test]
    fn completes_tables() {
        let t = case("SELECT * FROM ^");
        t.assert_labels(&["posts", "users"]);
    }

    #[test]
    fn ranks_tables_by_projected_columns() {
        let t = case("SELECT email FROM ^");
        t.assert_labels(&["users", "posts"]);
    }

    #[test]
    fn ranks_already_used_tables_lower() {
        let t = case("SELECT email FROM users u, ^");
        t.assert_labels(&["posts", "users"]);
    }

    #[test]
    fn completes_ctes() {
        let t = case("WITH cte AS (SELECT email FROM users) SELECT * FROM ^");
        t.assert_labels(&["cte", "posts", "users"]);
    }

    fn case(input: &str) -> CompletionTestResult {
        CompletionTest::from_input(input)
            .with_users_posts()
            .run_with(complete)
    }
}
