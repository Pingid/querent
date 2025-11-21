use querent_core::complete::DefaultCompleter;
use querent_core::complete::types::CompletionKind;
use querent_core::dialect::ansi;
use querent_core::test_utils::ScenarioComp;
use querent_core::test_utils::posts_schema;
use querent_core::test_utils::users_schema;

fn scenario() -> ScenarioComp {
    ScenarioComp::default()
        .completer(DefaultCompleter::default())
        .spec(ansi::SPEC.clone())
}

// ============================================================================
// Keyword Completions
// ============================================================================

// #[test]
// fn keyword_completes_partial_at_start() {
//     scenario().query("SELE^").starts(["SELECT"]).run();
// }

// // #[test]
// // fn keyword_includes_natural_join() {
// //     scenario()
// //         .query("SELECT * FROM users ^")
// //         .contains(["NATURAL JOIN", "INNER JOIN"])
// //         .run();
// // }

// // #[test]
// // fn with_suggests_recursive_after_with() {
// //     scenario().query("WITH ^").starts(["RECURSIVE"]).run();
// // }

// ============================================================================
// Column Completions - Basic
// ============================================================================

#[test]
fn column_suggests_from_all_tables_without_from() {
    // Without FROM clause, suggest columns from all tables in catalog
    scenario()
        .with((posts_schema(), users_schema()))
        .query("SELECT ^")
        .contains(["content", "email", "name", "posts.id", "title", "users.id"])
        .run();
}

#[test]
fn column_filters_by_already_selected() {
    // Prioritize columns from tables containing all already-selected
    scenario()
        .with((posts_schema(), users_schema()))
        .query("SELECT name, ^")
        .starts(["email", "id"])
        .run();
}

#[test]
fn column_restricts_to_from_tables() {
    // FROM clause restricts suggestions to columns from referenced tables
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT ^ FROM users")
        .starts(["email", "id", "name"])
        .run();
}

#[test]
fn column_suggests_after_distinct() {
    // Suggest columns after DISTINCT keyword
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT DISTINCT ^ FROM users")
        .starts(["email", "id", "name"])
        .run();
}

// ============================================================================
// Column Completions - Qualified Names
// ============================================================================

#[test]
fn column_completes_after_table_qualifier() {
    // Complete columns after qualified table name (table.^)
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT users.^ FROM users")
        .starts(["email", "id", "name"])
        .run();
}

#[test]
fn column_continues_qualified_syntax() {
    // Continue using qualified syntax when existing columns are qualified
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT users.name, ^ FROM users")
        .starts(["users.email", "users.id"])
        .run();
}

#[test]
fn column_uses_alias_when_qualified() {
    // Use alias in qualified completions when table has an alias
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT u.name, ^ FROM public.users u")
        .starts(["u.email", "u.id"])
        .run();
}

#[test]
fn column_resolves_from_schema_qualified_table() {
    // Resolve columns from schema-qualified table (schema.table)
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT ^ FROM public.users")
        .starts(["email", "id", "name"])
        .run();
}

#[test]
fn column_completes_after_alias_dot() {
    // Complete columns after alias and dot (alias.^)
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT u.^ FROM public.users u")
        .starts(["email", "id", "name"])
        .run();
}

#[test]
fn column_qualifies_with_aliases_for_multiple_tables() {
    // With multiple FROM sources and an existing qualified selection,
    // suggestions should be qualified using aliases and exclude already selected column
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT u.name, ^ FROM public.users u, public.posts p")
        .starts(["u.email", "u.id", "p.content", "p.id", "p.title"])
        .run();
}

#[test]
fn column_resolves_from_subquery_with_alias() {
    // Resolve columns from subquery when using alias qualifier
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT u.^ FROM (SELECT name, email FROM users) u")
        .starts(["email", "name"])
        .run();
}

#[test]
fn column_suggests_from_cte() {
    // Suggest columns from CTE (Common Table Expression)
    scenario()
        .with((users_schema(), posts_schema()))
        .query("WITH cte AS (SELECT id, name FROM public.users) SELECT ^ FROM")
        .starts(["id", "name"])
        .run();
}

#[test]
fn column_completes_from_cte_qualified() {
    // Complete columns from CTE using qualified name (cte.^)
    scenario()
        .with((users_schema(), posts_schema()))
        .query("WITH cte AS (SELECT id, name FROM public.users) SELECT cte.^ FROM cte")
        .starts(["id", "name"])
        .run();
}

#[test]
fn column_completes_from_cte_with_alias() {
    // Complete columns from CTE using an alias
    scenario()
        .with((users_schema(), posts_schema()))
        .query("WITH cte AS (SELECT id, name FROM public.users) SELECT u.^ FROM cte u")
        .starts(["id", "name"])
        .run();
}

#[test]
fn column_excludes_invalid_positions() {
    // No column completions without a comma and space
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT name ^")
        .none_of(CompletionKind::Column)
        .run();

    // // Qualifier doesn't match any relation in scope
    // let t = case("SELECT x.^ FROM users u").cache(users_posts()).run();
    // t.assert_col([]);

    // No completions after LIMIT keyword
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT * FROM users LIMIT ^")
        .none_of(CompletionKind::Column)
        .run();

    // No completions after OFFSET keyword
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT * FROM users LIMIT 10 OFFSET ^")
        .none_of(CompletionKind::Column)
        .run();
}

// ============================================================================
// Table Completions
// ============================================================================

#[test]
fn table_suggests_after_from() {
    // Suggest all tables after FROM keyword
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT * FROM ^")
        .starts(["posts", "users"])
        .run();
}

#[test]
fn table_suggests_after_from_multiline() {
    // Table suggestions work across multiple lines
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT 1\nFROM ^")
        .starts(["posts", "users"])
        .run();
}

#[test]
fn table_suggests_after_schema_qualifier() {
    // Suggest tables from specific schema after schema.^
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT * FROM public.^")
        .contains(["posts", "users"])
        .run();
}

// ============================================================================
// Join Completions
// ============================================================================

#[test]
fn join_suggests_tables_after_join() {
    // Suggest tables after JOIN keyword
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT * FROM users JOIN ^")
        .starts(["posts", "users"])
        .run();
}

#[test]
fn join_completes_qualified_columns() {
    // After table qualifier and dot, suggest columns from that table
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT * FROM users JOIN posts ON users.^")
        .starts(["email", "id", "name"])
        .run();
}

#[test]
fn join_completes_with_table_aliases() {
    // Use table aliases in JOIN ON completions
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT * FROM users u JOIN posts p ON u.^")
        .starts(["email", "id", "name"])
        .run();
}

// // ============================================================================
// // WHERE Clause Completions
// // ============================================================================

// #[test]
// fn where_suggests_columns_from_table() {
//     // Suggest columns from table in WHERE clause
//     scenario()
//         .with(users_schema())
//         .query("SELECT * FROM users WHERE ^")
//         .starts(["email", "id", "name"])
//         .run();
// }

// #[test]
// fn where_suggests_columns_after_logical_operator() {
//     // After AND/OR, suggest columns again
//     scenario()
//         .with(users_schema())
//         .query("SELECT * FROM users WHERE name = 'John' AND ^")
//         .starts(["email", "id", "name"])
//         .run();
// }

// #[test]
// fn where_excludes_after_comparison_operator() {
//     // No column suggestions immediately after comparison operators or values
//     scenario()
//         .with(users_schema())
//         .query("SELECT * FROM users WHERE name =^")
//         .query("SELECT * FROM users WHERE name = ^")
//         .query("SELECT * FROM users WHERE name = 'John'^")
//         .none_of(CompletionKind::Column)
//         .run();
// }

// // ============================================================================
// // ORDER BY Completions
// // ============================================================================

// #[test]
// fn order_by_suggests_columns() {
//     // Suggest all columns in ORDER BY clause
//     scenario()
//         .with(users_schema())
//         .query("SELECT * FROM users ORDER BY ^")
//         .starts(["email", "id", "name"])
//         .run();
// }

// #[test]
// fn order_by_excludes_already_ordered() {
//     // Exclude columns already used in ORDER BY
//     scenario()
//         .with(users_schema())
//         .query("SELECT * FROM users ORDER BY email, ^")
//         .starts(["id", "name"])
//         .run();
// }

// // // ============================================================================
// // // GROUP BY Completions
// // // ============================================================================

// // #[test]
// // fn group_by_suggests_non_aggregated_columns() {
// //     // Suggest non-aggregated columns from SELECT in GROUP BY
// //     let sql = "SELECT name, id FROM users GROUP BY ^";
// //     let t = case(sql).cache(users_posts()).run();
// //     t.assert_col(["id", "name"]);
// // }

// // #[test]
// // fn group_by_excludes_already_grouped() {
// //     // Exclude columns already in GROUP BY clause
// //     let sql = "SELECT name, id FROM users GROUP BY name, ^";
// //     let t = case(sql).cache(users_posts()).run();
// //     t.assert_col(["id"]);
// // }

// // // ============================================================================
// // // Subqueries & Correlation
// // // ============================================================================

// // #[test]
// // fn subquery_isolates_scope() {
// //     // Subquery WHERE should only see columns from inner query tables
// //     let t = case("SELECT * FROM users WHERE EXISTS (SELECT 1 FROM posts p
// // WHERE ^)")         .cache(users_posts())
// //         .run();
// //     t.assert_col(["content", "id", "title"]);
// // }

// // #[test]
// // fn subquery_completes_qualified_columns() {
// //     // Qualified columns in correlated subquery work correctly
// //     let t = case("SELECT * FROM users u WHERE EXISTS (SELECT 1 FROM posts p
// // WHERE p.^ = u.id)")         .cache(users_posts())
// //         .run();
// //     t.assert_col(["content", "id", "title"]);
// // }

// // // ============================================================================
// // // Operator Completions
// // // ============================================================================
// // // #[test]
// // // fn operator_suggests_comparison_operators() {
// // //     // Suggest comparison operators after column
// // //     let t = case("SELECT * FROM users WHERE id ^").run();
// // //     t.assert_includes_op([
// // //         "=", "!=", "<>", ">", "<", ">=", "<=", "IN", "NOT IN", "LIKE",
// // "IS", "IS NOT", //     ]);
// // // }

// // // #[test]
// // // fn operator_suggests_logical_operators_after_condition() {
// // //     // Suggest logical operators after complete condition
// // //     let t = case("SELECT * FROM users WHERE id = 1 ^")
// // //         .catalog(users_posts())
// // //         .run();
// // //     t.assert_includes_op(["AND", "OR"]);
// // // }

// // // ============================================================================
// // // CASE Expression Completions
// // // ============================================================================

// // #[test]
// // fn case_suggests_columns_after_when_condition() {
// //     // Suggest THEN after WHEN condition
// //     let t = case("SELECT CASE WHEN ^").cache(users_posts()).run();
// //     // Duplicate "id" columns show with qualified names
// //     t.assert_col(["content", "email", "name", "posts.id", "title",
// // "users.id"]); }
