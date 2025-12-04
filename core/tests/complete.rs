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

#[test]
fn keyword_completes_partial_at_start() {
    scenario().query("SELE^").starts(["SELECT"]).run();
}

#[test]
fn keyword_includes_natural_join() {
    scenario()
        .query("SELECT * FROM users ^")
        .contains(["NATURAL JOIN", "INNER JOIN"])
        .run();
}

#[test]
fn with_suggests_recursive_after_with() {
    scenario().query("WITH ^").starts(["RECURSIVE"]).run();
}

// ============================================================================
// Column Completions - Basic
// ============================================================================

#[test]
fn column_suggests_from_all_tables_without_from() {
    // Without FROM clause, suggest columns from all tables in catalog
    scenario()
        .query("SELECT ^")
        .with((posts_schema(), users_schema()))
        .contains(["content", "email", "name", "posts.id", "title", "users.id"])
        .run();
}

#[test]
fn column_filters_by_already_selected() {
    // Prioritize columns from tables containing all already-selected
    scenario()
        .query("SELECT name, ^")
        .with((posts_schema(), users_schema()))
        .starts(["email", "id"])
        .run();
}

#[test]
fn column_restricts_to_from_tables() {
    // FROM clause restricts suggestions to columns from referenced tables
    scenario()
        .query("SELECT ^ FROM users")
        .with((users_schema(), posts_schema()))
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
fn column_qualified_deprioritizes_already_selected_for_same_alias() {
    // When adding more columns for the same alias, de-prioritize ones already selected
    scenario()
        .query("SELECT u.name, u.^ FROM users u JOIN posts p ON p.id = u.id")
        .with((users_schema(), posts_schema()))
        .starts(["email", "id"])
        .run();
}

#[test]
fn column_completes_after_table_qualifier() {
    // Complete columns after qualified table name (table.^)
    scenario()
        .query("SELECT users.^ FROM users")
        .with((users_schema(), posts_schema()))
        .starts(["email", "id", "name"])
        .run();
}

#[test]
fn column_continues_qualified_syntax() {
    // Continue using qualified syntax when existing columns are qualified
    scenario()
        .query("SELECT users.name, ^ FROM users")
        .with((users_schema(), posts_schema()))
        .starts(["users.email", "users.id"])
        .run();
}

#[test]
fn column_uses_alias_when_qualified() {
    // Use alias in qualified completions when table has an alias
    scenario()
        .query("SELECT u.name, ^ FROM public.users u")
        .with((users_schema(), posts_schema()))
        .starts(["u.email", "u.id"])
        .run();
}

#[test]
fn column_resolves_from_schema_qualified_table() {
    // Resolve columns from schema-qualified table (schema.table)
    scenario()
        .query("SELECT ^ FROM public.users")
        .with((users_schema(), posts_schema()))
        .starts(["email", "id", "name"])
        .run();
}

#[test]
fn column_completes_after_alias_dot() {
    // Complete columns after alias and dot (alias.^)
    scenario()
        .query("SELECT u.^ FROM public.users u")
        .with((users_schema(), posts_schema()))
        .starts(["email", "id", "name"])
        .run();
}

#[test]
fn column_qualifies_with_aliases_for_multiple_tables() {
    // With multiple FROM sources and an existing qualified selection,
    // suggestions should be qualified using aliases and deprioritize already selected columns
    scenario()
        .query("SELECT u.name, ^ FROM public.users u, public.posts p")
        .with((users_schema(), posts_schema()))
        .starts(["u.email", "u.id", "p.content", "p.id", "p.title"])
        .run();
}

#[test]
fn column_resolves_from_subquery_with_alias() {
    // Resolve columns from subquery when using alias qualifier
    scenario()
        .query("SELECT u.^ FROM (SELECT name, email FROM users) u")
        .with((users_schema(), posts_schema()))
        .starts(["email", "name"])
        .run();
}

#[test]
fn column_suggests_from_cte() {
    // Suggest columns from CTE (Common Table Expression)
    scenario()
        .query("WITH cte AS (SELECT id, name FROM public.users) SELECT ^ FROM")
        .with((users_schema(), posts_schema()))
        .starts(["id", "name"])
        .run();
}

#[test]
fn column_completes_from_cte_qualified() {
    // Complete columns from CTE using qualified name (cte.^)
    scenario()
        .query("WITH cte AS (SELECT id, name FROM public.users) SELECT cte.^ FROM cte")
        .with((users_schema(), posts_schema()))
        .starts(["id", "name"])
        .run();
}

#[test]
fn column_completes_from_cte_with_alias() {
    // Complete columns from CTE using an alias
    scenario()
        .query("WITH cte AS (SELECT id, name FROM public.users) SELECT u.^ FROM cte u")
        .with((users_schema(), posts_schema()))
        .starts(["id", "name"])
        .run();
}

#[test]
fn column_suggests_from_subquery_output_in_select_list() {
    // Selecting from a derived table should suggest its projected columns
    scenario()
        .query("SELECT ^ FROM (SELECT id, name FROM users) u")
        .with(users_schema())
        .starts(["id", "name"])
        .run();
}

#[test]
fn column_prioritizes_alias_used_in_where_for_select() {
    // When WHERE focuses on a specific alias, prefer that alias in SELECT
    scenario()
        .query("SELECT ^ FROM users u JOIN posts p ON u.id = p.id WHERE u.email = 'tom@gmail.com'")
        .with((users_schema(), posts_schema()))
        .starts(["u.email", "u.id", "u.name", "p.content", "p.id", "p.title"])
        .run();
}

#[test]
fn column_prioritizes_columns_from_last_used_alias_in_select_list() {
    // After selecting from one alias, prefer more columns from that alias
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT p.title, ^ FROM users u JOIN posts p ON p.id = u.id")
        .starts(["p.content", "p.id", "u.email", "u.id", "u.name"])
        .run();
}

#[test]
fn column_ignores_inner_scope_when_completing_outer_select_list() {
    // Outer SELECT shouldn't be polluted by inner subquery scope.
    // Even though inner subquery references `posts p`, completions should only
    // show columns from outer FROM clause (`users u`).
    scenario()
        .query("SELECT ^, (SELECT 1 FROM posts p WHERE p.id = u.id) FROM users u")
        .with((users_schema(), posts_schema()))
        // Only users columns should appear, not posts columns
        .starts(["email", "id", "name"])
        .none_of(["content", "title", "posts.id", "p.id"])
        .run();
}

#[test]
fn column_isolates_scope_for_inner_select_list() {
    // Inner SELECT should only see its own FROM (scope isolation)
    scenario()
        .query("SELECT (SELECT ^ FROM posts p WHERE p.id = u.id) FROM users u")
        .with((users_schema(), posts_schema()))
        .starts(["content", "id", "title"])
        .run();
}

#[test]
fn column_respects_cte_shadowing_base_table() {
    // A CTE with same name as a base table should define the visible columns
    scenario()
        .query("WITH users AS (SELECT id FROM public.users) SELECT ^ FROM users")
        .with(users_schema())
        .starts(["id"])
        .run();
}

#[test]
fn column_prioritizes_cte_alias_used_in_where_for_select() {
    // When selecting from a CTE with alias, and WHERE uses that alias,
    // prioritize its columns in SELECT completions
    scenario()
        .query(
            "WITH active_users AS (SELECT id, email FROM users) SELECT ^ FROM active_users au WHERE au.email LIKE '%@example.com'",
        )
        .with(users_schema())
        .starts(["au.email", "au.id"])
        .run();
}

#[test]
fn column_prioritizes_grouping_keys_in_select() {
    // In a grouped query, surface grouping keys early for additional selects
    scenario()
        .query("SELECT name, COUNT(*), ^ FROM users GROUP BY name HAVING COUNT(*) > 1")
        .with(users_schema())
        .starts(["name"])
        .run();
}

#[test]
fn column_deprioritizes_invalid_positions() {
    // No column completions without a comma and space
    scenario()
        .query("SELECT name ^")
        .with((users_schema(), posts_schema()))
        .none_of(CompletionKind::Column)
        .run();

    // No completions after LIMIT keyword
    scenario()
        .query("SELECT * FROM users LIMIT ^")
        .with((users_schema(), posts_schema()))
        .none_of(CompletionKind::Column)
        .run();

    // No completions after OFFSET keyword
    scenario()
        .query("SELECT * FROM users LIMIT 10 OFFSET ^")
        .with((users_schema(), posts_schema()))
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
        .query("SELECT * FROM ^")
        .with((users_schema(), posts_schema()))
        .starts(["posts", "users"])
        .run();
}

#[test]
fn table_suggests_after_from_multiline() {
    // Table suggestions work across multiple lines
    scenario()
        .query("SELECT 1\nFROM ^")
        .with((users_schema(), posts_schema()))
        .starts(["posts", "users"])
        .run();
}

#[test]
fn table_suggests_after_schema_qualifier() {
    // Suggest tables from specific schema after schema.^
    scenario()
        .query("SELECT * FROM public.^")
        .with((users_schema(), posts_schema()))
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
        .query("SELECT * FROM users JOIN ^")
        .with((users_schema(), posts_schema()))
        .starts(["posts", "users"])
        .run();
}

#[test]
fn join_completes_qualified_columns() {
    // After table qualifier and dot, suggest columns from that table
    scenario()
        .query("SELECT * FROM users JOIN posts ON users.^")
        .with((users_schema(), posts_schema()))
        .starts(["email", "id", "name"])
        .run();
}

#[test]
fn join_completes_with_table_aliases() {
    // Use table aliases in JOIN ON completions
    scenario()
        .query("SELECT * FROM users u JOIN posts p ON u.^")
        .with((users_schema(), posts_schema()))
        .starts(["email", "id", "name"])
        .run();
}

// ============================================================================
// WHERE Clause Completions
// ============================================================================

#[test]
fn where_suggests_columns_from_table() {
    // Suggest columns from table in WHERE clause
    scenario()
        .query("SELECT * FROM users WHERE ^")
        .with((users_schema(), posts_schema()))
        .starts(["email", "id", "name"])
        .run();
}

#[test]
fn where_suggests_columns_after_logical_operator() {
    // After AND/OR, suggest columns again
    scenario()
        .query("SELECT * FROM users WHERE name = 'John' AND ^")
        .with(users_schema())
        .starts(["email", "id", "name"])
        .run();
}

#[test]
fn where_deprioritizes_after_comparison_operator() {
    // No column suggestions immediately after comparison operators or values
    scenario()
        .query("SELECT * FROM users WHERE name =^")
        .query("SELECT * FROM users WHERE name = ^")
        .query("SELECT * FROM users WHERE name = 'John'^")
        .with(users_schema())
        .none_of(CompletionKind::Column)
        .run();
}

// ============================================================================
// ORDER BY Completions
// ============================================================================

#[test]
fn order_by_suggests_columns() {
    // Suggest all columns in ORDER BY clause
    scenario()
        .query("SELECT * FROM users ORDER BY ^")
        .with(users_schema())
        .starts(["email", "id", "name"])
        .run();
}

#[test]
fn order_by_deprioritizes_already_ordered() {
    // Exclude columns already used in ORDER BY
    scenario()
        .query("SELECT * FROM users ORDER BY email, ^")
        .with(users_schema())
        .starts(["id", "name"])
        .run();
}

// ============================================================================
// GROUP BY Completions
// ============================================================================

#[test]
fn group_by_suggests_non_aggregated_columns() {
    // Suggest non-aggregated columns from SELECT in GROUP BY
    scenario()
        .query("SELECT name, id FROM users GROUP BY ^")
        .with((users_schema(), posts_schema()))
        .starts(["id", "name"])
        .run();
}

#[test]
fn group_by_deprioritizes_already_grouped() {
    // De-prioritize columns already in GROUP BY clause
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT name, id FROM users GROUP BY name, ^")
        .starts(["id"])
        .run();
}

// ============================================================================
// Subqueries & Correlation
// ============================================================================

#[test]
fn subquery_isolates_scope() {
    // Subquery WHERE should only see columns from inner query tables
    scenario()
        .with(posts_schema())
        .query("SELECT * FROM users WHERE EXISTS (SELECT 1 FROM posts p WHERE ^)")
        .starts(["content", "id", "title"])
        .run();
}

#[test]
fn subquery_completes_qualified_columns() {
    // Qualified columns in correlated subquery work correctly
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT * FROM users u WHERE EXISTS (SELECT 1 FROM posts p WHERE p.^ = u.id)")
        .starts(["content", "id", "title"])
        .run();
}

// // ============================================================================
// // Operator Completions
// // ============================================================================
// #[test]
// fn operator_suggests_comparison_operators() {
//     // Suggest comparison operators after column
//     scenario()
//         .with(users_schema())
//         .query("SELECT * FROM users WHERE id ^")
//         .contains([
//             "=", "!=", "<>", ">", "<", ">=", "<=", "IN", "NOT IN", "LIKE", "IS", "IS NOT",
//         ])
//         .run();
// }

// ============================================================================
// CASE Expression Completions
// ============================================================================

#[test]
fn case_suggests_columns_after_when_condition() {
    // Suggest THEN after WHEN condition
    scenario()
        .with((users_schema(), posts_schema()))
        .query("SELECT CASE WHEN ^")
        .contains(["content", "email", "name", "posts.id", "title", "users.id"])
        .run();
}
