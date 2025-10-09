use querent_core::{
    catalog::{InMemoryCatalog, schema},
    dialect::{Ansi, Dialect},
    doc::Content,
    engine::{Completion, CompletionKind, Engine},
};

mod common;
use common::*;

// ============================================================================
// Keyword Completions
// ============================================================================

#[test]
fn keyword_completes_partial_at_start() {
    // Complete partial keyword at statement start
    let t: TestCase = case("SELE^").run();
    t.assert_kw_starts_with(["SELECT"]);
    t.assert_apply("SELECT ");
}

#[test]
fn keyword_suggests_from_after_select() {
    // Suggest FROM keyword after SELECT clause
    let t: TestCase = case("SELECT name ^").run();
    t.assert_includes_kw(["FROM"]);
}

#[test]
fn keyword_suggests_clauses_after_from() {
    // After FROM clause, suggest common trailing keywords and clauses
    let t = case("SELECT * FROM users ^").run();
    t.assert_includes_kw(["JOIN", "WHERE", "GROUP BY", "ORDER BY", "LIMIT"]);
}

#[test]
fn keyword_suggests_all_join_types() {
    // Suggest all JOIN type variants as multi-word keywords
    let t = case("SELECT * FROM users ^").run();
    t.assert_includes_kw([
        "LEFT JOIN",
        "RIGHT JOIN",
        "FULL JOIN",
        "INNER JOIN",
        "OUTER JOIN",
        "CROSS JOIN",
        "NATURAL JOIN",
    ]);
    let t = case("SELECT * FROM users LE^").run();
    t.assert_kw_starts_with(["LEFT JOIN"]);
}

#[test]
fn keyword_suggests_set_operations() {
    // Suggest set operation keywords after a complete SELECT statement
    let t = case("SELECT id FROM users ^").catalog(users_posts()).run();
    t.assert_includes_kw(["UNION", "UNION ALL", "INTERSECT", "EXCEPT"]);
}

#[test]
fn keyword_suggests_order_by_modifiers() {
    // Suggest sort direction (ASC/DESC) and null ordering modifiers
    let t = case("SELECT * FROM users ORDER BY name ^")
        .catalog(users_posts())
        .run();
    t.assert_includes_kw(["ASC", "DESC", "NULLS FIRST", "NULLS LAST"]);
}

#[test]
fn keyword_includes_natural_join() {
    // Suggest NATURAL JOIN alongside other join types
    let t = case("SELECT * FROM users ^").catalog(users_posts()).run();
    t.assert_includes_kw(["NATURAL JOIN", "INNER JOIN"]);
}

#[test]
fn with_suggests_recursive_after_with() {
    // Suggest RECURSIVE after WITH
    let t = case("WITH ^").run();
    t.assert_includes_kw(["RECURSIVE"]);
}

// ============================================================================
// Column Completions - Basic
// ============================================================================

#[test]
fn column_suggests_from_all_tables_without_from() {
    // Without FROM clause, suggest columns from all tables in catalog
    let t = case("SELECT ^").catalog(users_posts()).run();
    t.assert_col(["content", "email", "id", "id", "name", "title"]);
    t.assert_col_source([
        "public.posts",
        "public.users",
        "public.users",
        "public.posts",
        "public.users",
        "public.posts",
    ]);
}

#[test]
fn column_filters_by_already_selected() {
    // Only suggest columns from tables containing all already-selected columns
    let t = case("SELECT name, ^").catalog(users_posts()).run();
    t.assert_col(["email", "id"]);
}

#[test]
fn column_restricts_to_from_tables() {
    // FROM clause restricts suggestions to columns from referenced tables only
    let t = case("SELECT ^ FROM users").catalog(users_posts()).run();
    t.assert_col(["email", "id", "name"]);
}

#[test]
fn column_suggests_after_distinct() {
    // Suggest columns after DISTINCT keyword
    let t = case("SELECT DISTINCT ^ FROM users")
        .catalog(users_posts())
        .run();
    t.assert_col(["email", "id", "name"]);
}

#[test]
fn having_suggests_after_group_by() {
    // Suggest HAVING after GROUP BY
    let t = case("SELECT COUNT(*) FROM users GROUP BY name ^")
        .catalog(users_posts())
        .run();
    t.assert_includes_kw(["HAVING", "ORDER BY", "LIMIT"]);
}

// ============================================================================
// Column Completions - Qualified Names
// ============================================================================

#[test]
fn column_completes_after_table_qualifier() {
    // Complete columns after qualified table name (table.^)
    let t = case("SELECT users.^ FROM users")
        .catalog(users_posts())
        .run();
    t.assert_col(["email", "id", "name"]);
}

#[test]
fn column_continues_qualified_syntax() {
    // Continue using qualified syntax when existing columns are qualified
    let t = case("SELECT users.name, ^ FROM users")
        .catalog(users_posts())
        .run();
    t.assert_col(["email", "id"]);
    t.assert_apply("SELECT users.name, users.email FROM users");
}

#[test]
fn column_uses_alias_when_qualified() {
    // Use alias in qualified completions when table has an alias
    let t = case("SELECT u.name, ^ FROM public.users u")
        .catalog(users_posts())
        .run();
    t.assert_apply("SELECT u.name, u.email FROM public.users u");
}

#[test]
fn column_resolves_from_schema_qualified_table() {
    // Resolve columns from schema-qualified table (schema.table)
    let t = case("SELECT ^ FROM public.users")
        .catalog(users_posts())
        .run();
    t.assert_col(["email", "id", "name"]);
}

#[test]
fn column_completes_after_alias_dot() {
    // Complete columns after alias and dot (alias.^)
    let t = case("SELECT u.^ FROM public.users u")
        .catalog(users_posts())
        .run();
    t.assert_col(["email", "id", "name"]);
}

#[test]
fn column_qualifies_with_aliases_for_multiple_tables() {
    // With multiple FROM sources and an existing qualified selection,
    // suggestions should be qualified using aliases and exclude already selected column
    let t = case("SELECT u.name, ^ FROM public.users u, public.posts p")
        .catalog(users_posts())
        .run();

    // Order is deterministic (label asc, then source), but we assert a subsequence to be robust
    t.assert_kind_contains_in_order(
        |k| matches!(k, CompletionKind::Column(_)),
        ["content", "email", "id", "id", "title"],
    );
    t.assert_apply("SELECT u.name, p.content FROM public.users u, public.posts p");
}

#[test]
fn column_resolves_from_subquery_with_alias() {
    // Resolve columns from subquery when using alias qualifier
    let t = case("SELECT u.^ FROM (SELECT name, email FROM users) u")
        .catalog(users_posts())
        .run();
    t.assert_col(["email", "name"]);
    t.assert_col_source(["public.users", "public.users"]);
}

#[test]
fn column_suggests_from_cte() {
    // Suggest columns from CTE (Common Table Expression)
    let sql = "WITH cte AS (SELECT id, name FROM public.users) SELECT ^ FROM cte";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col(["id", "name"]);
    t.assert_col_source(["public.users", "public.users"]);
}

#[test]
fn column_completes_from_cte_qualified() {
    // Complete columns from CTE using qualified name (cte.^)
    let sql = "WITH cte AS (SELECT id, name FROM public.users) SELECT cte.^ FROM cte";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col(["id", "name"]);
    t.assert_col_source(["public.users", "public.users"]);
}

#[test]
fn column_completes_from_cte_with_alias() {
    // Complete columns from CTE using an alias
    let sql = "WITH cte AS (SELECT id, name FROM public.users) SELECT u.^ FROM cte u";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col(["id", "name"]);
    t.assert_col_source(["public.users", "public.users"]);
}

#[test]
fn column_excludes_invalid_positions() {
    // No column completions without a comma and space
    let t1 = case("SELECT name ^").catalog(users_posts()).run();
    t1.assert_col([]);

    // No completions after a comma without a space
    let t = case("SELECT name,^").catalog(users_posts()).run();
    t.assert_col([]);

    // Qualifier doesn't match any relation in scope
    let t = case("SELECT x.^ FROM users u").catalog(users_posts()).run();
    t.assert_col([]);

    // No completions after LIMIT keyword
    let t = case("SELECT * FROM users LIMIT ^")
        .catalog(users_posts())
        .run();
    t.assert_col([]);

    // No completions after OFFSET keyword
    let t = case("SELECT * FROM users LIMIT 10 OFFSET ^")
        .catalog(users_posts())
        .run();
    t.assert_col([]);
}

// ============================================================================
// Table Completions
// ============================================================================

#[test]
fn table_suggests_after_from() {
    // Suggest all tables after FROM keyword
    let t = case("SELECT * FROM ^").catalog(users_posts()).run();
    t.assert_table(["posts", "users"]);
}

#[test]
fn table_suggests_after_from_multiline() {
    // Table suggestions work across multiple lines
    let t = case("SELECT 1\nFROM ^").catalog(users_posts()).run();
    t.assert_table(["posts", "users"]);
}

#[test]
fn table_shows_duplicates_from_different_schemas() {
    // Same table name in different schemas should show both with qualified sources
    let cat = CatalogBuilder::new()
        .table("public", "users", &["id"])
        .table("analytics", "users", &["id"])
        .build();
    let t = case("SELECT * FROM ^").catalog(cat).run();
    t.assert_table(["users", "users"]);
    t.assert_table_source(["analytics", "public"]);
}

#[test]
fn table_suggests_after_schema_qualifier() {
    // Suggest tables from specific schema after schema.^
    let t = case("SELECT * FROM public.^").catalog(users_posts()).run();
    t.assert_table(["posts", "users"]);
}

#[test]
fn table_excludes_already_referenced() {
    // After comma, exclude already-referenced tables
    let t = case("SELECT * FROM users, ^").catalog(users_posts()).run();
    t.assert_table(["posts"]);
}

// ============================================================================
// Join Completions
// ============================================================================

#[test]
fn join_suggests_tables_after_join() {
    // Suggest tables after JOIN keyword
    let sql = "SELECT * FROM users JOIN ^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_table(["posts", "users"]);
}

#[test]
fn join_suggests_columns_in_on_clause() {
    // Suggest columns from both tables in JOIN ON clause
    let sql = "SELECT * FROM users JOIN posts ON ^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col(["content", "email", "id", "id", "name", "title"]);
}

#[test]
fn join_suggests_columns_after_logical_operator() {
    // After logical operator (AND/OR), suggest columns again
    let sql = "SELECT * FROM users JOIN posts ON users.id = posts.id AND ^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col(["content", "email", "id", "id", "name", "title"]);
}

#[test]
fn join_completes_qualified_columns() {
    // After table qualifier and dot, suggest columns from that table
    let sql = "SELECT * FROM users JOIN posts ON users.^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col(["email", "id", "name"]);
}

#[test]
fn join_suggests_tables_for_multiple_joins() {
    // With multiple joins, suggest all available tables
    let sql = "SELECT * FROM users JOIN posts ON users.id = posts.id JOIN ^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_table(["posts", "users"]);
}

#[test]
fn join_completes_with_table_aliases() {
    // Use table aliases in JOIN ON completions
    let sql = "SELECT * FROM users u JOIN posts p ON u.^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col(["email", "id", "name"]);
}

#[test]
fn join_supports_left_outer() {
    // LEFT JOIN works the same as INNER JOIN
    let sql = "SELECT * FROM users LEFT JOIN ^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_table(["posts", "users"]);
}

#[test]
fn join_supports_inner() {
    // INNER JOIN works the same as JOIN
    let sql = "SELECT * FROM users INNER JOIN posts ON ^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col(["content", "email", "id", "id", "name", "title"]);
}

#[test]
fn join_suggests_common_columns_in_using() {
    // USING clause suggests only columns that exist in both tables
    let t = case("SELECT * FROM users JOIN posts USING (^)")
        .catalog(users_posts())
        .run();
    t.assert_col(["id"]);
}

// ============================================================================
// WHERE Clause Completions
// ============================================================================

#[test]
fn where_suggests_columns_from_table() {
    // Suggest columns from table in WHERE clause
    let sql = "SELECT * FROM users WHERE ^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col(["email", "id", "name"]);
}

#[test]
fn where_suggests_columns_after_logical_operator() {
    // After AND/OR, suggest columns again
    let sql = "SELECT * FROM users WHERE name = 'John' AND ^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col(["email", "id", "name"]);
}

#[test]
fn where_excludes_after_comparison_operator() {
    // No column suggestions immediately after comparison operators or values
    let sql = "SELECT * FROM users WHERE name =^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col([]);
    let sql = "SELECT * FROM users WHERE name = ^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col([]);
    let sql = "SELECT * FROM users WHERE name = 'John'^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col([]);
}

// ============================================================================
// ORDER BY Completions
// ============================================================================

#[test]
fn order_by_suggests_columns() {
    // Suggest all columns in ORDER BY clause
    let t = case("SELECT * FROM users ORDER BY ^")
        .catalog(users_posts())
        .run();
    t.assert_col(["email", "id", "name"]);
}

#[test]
fn order_by_excludes_already_ordered() {
    // Exclude columns already used in ORDER BY
    let t = case("SELECT * FROM users ORDER BY name, ^")
        .catalog(users_posts())
        .run();
    t.assert_col(["email", "id"]);
}

// ============================================================================
// GROUP BY Completions
// ============================================================================

#[test]
fn group_by_suggests_non_aggregated_columns() {
    // Suggest non-aggregated columns from SELECT in GROUP BY
    let sql = "SELECT name, id FROM users GROUP BY ^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col(["id", "name"]);
}

#[test]
fn group_by_excludes_already_grouped() {
    // Exclude columns already in GROUP BY clause
    let sql = "SELECT name, id FROM users GROUP BY name, ^";
    let t = case(sql).catalog(users_posts()).run();
    t.assert_col(["id"]);
}

// ============================================================================
// Subqueries & Correlation
// ============================================================================

#[test]
fn subquery_isolates_scope() {
    // Subquery WHERE should only see columns from inner query tables
    let t = case("SELECT * FROM users WHERE EXISTS (SELECT 1 FROM posts p WHERE ^)")
        .catalog(users_posts())
        .run();
    t.assert_col(["content", "id", "title"]);
}

#[test]
fn subquery_completes_qualified_columns() {
    // Qualified columns in correlated subquery work correctly
    let t = case("SELECT * FROM users u WHERE EXISTS (SELECT 1 FROM posts p WHERE p.^ = u.id)")
        .catalog(users_posts())
        .run();
    t.assert_col(["content", "id", "title"]);
}

// ============================================================================
// Operator Completions
// ============================================================================
#[test]
fn operator_suggests_comparison_operators() {
    // Suggest comparison operators after column
    let t = case("SELECT * FROM users WHERE id ^").run();
    t.assert_includes_op([
        "=", "!=", "<>", ">", "<", ">=", "<=", "IN", "NOT IN", "LIKE", "IS", "IS NOT",
    ]);
}

#[test]
fn operator_suggests_logical_operators_after_condition() {
    // Suggest logical operators after complete condition
    let t = case("SELECT * FROM users WHERE id = 1 ^")
        .catalog(users_posts())
        .run();
    t.assert_includes_op(["AND", "OR"]);
}

// ============================================================================
// CASE Expression Completions
// ============================================================================

#[test]
fn case_suggests_when_after_case() {
    // Suggest WHEN after CASE
    let t = case("SELECT CASE ^").run();
    t.assert_includes_kw(["WHEN"]);
}

#[test]
fn case_suggests_columns_after_when_condition() {
    // Suggest THEN after WHEN condition
    let t = case("SELECT CASE WHEN ^").catalog(users_posts()).run();
    t.assert_col(["content", "email", "id", "id", "name", "title"]);
}

#[test]
fn case_suggests_then_after_when_condition() {
    // Suggest THEN after WHEN condition
    let t = case("SELECT CASE WHEN id = 1 ^").run();
    t.assert_includes_kw(["THEN"]);
}

#[test]
fn case_suggests_when_else_end_after_then() {
    // Suggest WHEN, ELSE, END after THEN value
    let t = case("SELECT CASE WHEN id = 1 THEN 'one' ^").run();
    t.assert_includes_kw(["WHEN", "ELSE", "END"]);
}

// ============================================================================
// Test Utilities
// ============================================================================

fn users_posts() -> InMemoryCatalog {
    CatalogBuilder::new()
        .table("public", "users", &["id", "name", "email"])
        .table("public", "posts", &["id", "title", "content"])
        .build()
}

fn case(input: &str) -> CompletionTester<Ansi> {
    let (input, pos) = with_caret_cursor(input);
    CompletionTester::new(Ansi, input, pos)
}

struct TestCase {
    pub sql: String,
    pub completions: Vec<Completion>,
}

impl TestCase {
    /// Assert that the first completions have the expected Column labels (exact order).
    fn assert_col<const N: usize>(&self, expected: [&str; N]) {
        let labels: Vec<_> = self
            .completions
            .iter()
            .filter(|c| matches!(c.kind, CompletionKind::Column(_)))
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels, expected);
    }
    /// Assert the Column source metadata (table or schema.table) matches in order.
    fn assert_col_source<const N: usize>(&self, expected: [&str; N]) {
        let labels: Vec<_> = self
            .completions
            .iter()
            .filter_map(|c| match &c.kind {
                CompletionKind::Column(col_completion) => col_completion.qualifier.clone(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, expected);
    }

    fn assert_table_source<const N: usize>(&self, expected: [&str; N]) {
        let labels: Vec<_> = self
            .completions
            .iter()
            .filter_map(|c| match &c.kind {
                CompletionKind::Table(table_completion) => table_completion.qualifier.clone(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, expected);
    }

    /// Assert the first N keyword labels, preserving order.
    fn assert_kw_starts_with<const N: usize>(&self, expected: [&str; N]) {
        let labels: Vec<_> = self
            .completions
            .iter()
            .filter(|c| c.kind == CompletionKind::Keyword)
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels[0..N], expected);
    }

    fn assert_includes_kw<const N: usize>(&self, expected: [&str; N]) {
        let labels: Vec<_> = self
            .completions
            .iter()
            .filter(|c| matches!(c.kind, CompletionKind::Keyword))
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>();
        for e in expected {
            if !labels.contains(&e) {
                eprintln!("Expected '{}' not found in keywords: {:?}", e, labels);
            }
            assert!(labels.contains(&e));
        }
    }

    fn assert_includes_op<const N: usize>(&self, expected: [&str; N]) {
        let labels: Vec<_> = self
            .completions
            .iter()
            .filter(|c| matches!(c.kind, CompletionKind::Operator))
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>();
        for e in expected {
            if !labels.contains(&e) {
                eprintln!("Expected '{}' not found in operators: {:?}", e, labels);
            }
            assert!(labels.contains(&e));
        }
    }

    /// Assert table labels (exact order).
    fn assert_table<const N: usize>(&self, expected: [&str; N]) {
        let labels: Vec<_> = self
            .completions
            .iter()
            .filter(|c| matches!(c.kind, CompletionKind::Table(_)))
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels, expected);
    }
    /// Apply the completion found at the given index.
    fn apply_nth(&self, idx: usize) -> String {
        let c = &self.completions[idx];
        let mut out = String::with_capacity(
            self.sql.len() - (c.replace.end - c.replace.start) + c.insert_text.len(),
        );
        out.push_str(&self.sql[..c.replace.start]);
        out.push_str(&c.insert_text);
        out.push_str(&self.sql[c.replace.end..]);
        out
    }
    pub(crate) fn assert_apply(&self, expected: &str) {
        assert_eq!(self.apply_nth(0), expected);
    }

    // ---- New helpers ----

    /// Assert that ALL completion labels (for a given kind) contain the provided set,
    /// in the exact order they appear in results. Useful when extra items may trail.
    fn assert_kind_contains_in_order<const N: usize>(
        &self,
        kind_pred: impl Fn(&CompletionKind) -> bool,
        expected: [&str; N],
    ) {
        let labels: Vec<_> = self
            .completions
            .iter()
            .filter(|c| kind_pred(&c.kind))
            .map(|c| c.label.as_str())
            .collect();
        let mut j = 0;
        for label in labels {
            if j < expected.len() && label == expected[j] {
                j += 1;
            }
        }
        assert_eq!(
            j,
            expected.len(),
            "Expected subsequence {:?} not found in order",
            expected
        );
    }
}

struct CompletionTester<D> {
    catalog: InMemoryCatalog,
    dialect: D,
    input: String,
    cursor: usize,
}

impl<D: Dialect> CompletionTester<D> {
    fn new(dialect: D, input: impl Into<String>, cursor: usize) -> Self {
        Self {
            catalog: InMemoryCatalog::new(),
            dialect,
            input: input.into(),
            cursor,
        }
    }
    fn catalog(mut self, catalog: InMemoryCatalog) -> Self {
        self.catalog = catalog;
        self
    }
    fn run(self) -> TestCase {
        let mut doc = Content::default();
        doc.set_content(&self.input);
        doc.set_cursor(self.cursor);
        let engine = Engine::new(self.catalog, self.dialect);
        let completions = futures::executor::block_on(engine.complete(&doc));
        TestCase {
            sql: self.input,
            completions,
        }
    }
}

struct CatalogBuilder(InMemoryCatalog);
impl CatalogBuilder {
    pub(crate) fn new() -> Self {
        Self(InMemoryCatalog::new())
    }
    pub(crate) fn table(mut self, schema: &str, name: &str, cols: &[&str]) -> Self {
        let mut table = schema::Table::new(name);
        table.columns = cols.iter().map(|c| schema::Column::new(*c)).collect();
        self.0.add_table(schema, table);
        self
    }
    pub(crate) fn build(self) -> InMemoryCatalog {
        self.0
    }
}
