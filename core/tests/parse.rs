use querent_core::ast;
use querent_core::dialect::DialectSpec;
use querent_core::dialect::ansi;
use querent_core::dialect::postgres;
use querent_core::dialect::sqlite;
use querent_core::lex::lex;
use querent_core::parse::v2::WinnowParser;

mod common;
use common::ast::AstDisplay;

// ---------------- Test: Statement ----------------
#[test]
fn statement_partial_and_query() {
    let ast = statement("", &ansi::SPEC);
    assert!(matches!(ast, ast::Statement::Partial(_)));
    let ast = statement("SEL", &ansi::SPEC);
    assert!(matches!(ast, ast::Statement::Partial(_)));
    let ast = statement("SELECT", &ansi::SPEC);
    assert!(matches!(ast, ast::Statement::Query(_)));
}

#[test]
fn ans_complete() {
    // panic!("{:#?}", statement("SELECT *", ANSI_COMPLIANT_SPECS[0]));
    let inputs = [
        // Literals & expressions
        "SELECT 1",
        "SELECT NULL",
        "SELECT TRUE",
        "SELECT FALSE",
        "SELECT a + b, c * d, e / f, g - h",
        "SELECT (a + b) * c",
        "SELECT -a, +b",
        // CASE expressions
        "SELECT CASE WHEN age > 18 THEN 'adult' ELSE 'child' END AS category",
        "SELECT CASE status WHEN 'active' THEN 1 WHEN 'inactive' THEN 0 ELSE -1 END",
        // CAST expressions
        "SELECT CAST(x AS INTEGER)",
        "SELECT CAST(y AS VARCHAR(255))",
        "SELECT CAST(z AS NUMERIC(10, 2))",
        // Typed literals
        "SELECT DATE '2024-01-01'",
        "SELECT TIME '12:30:00'",
        "SELECT TIMESTAMP '2024-01-01 12:30:00'",
        "SELECT INTERVAL '1 day'",
        // ROW constructor
        "SELECT ROW(1, 2, 3)",
        "SELECT ROW(a, b, c) FROM t",
        // DISTINCT / alias
        "SELECT DISTINCT a, b",
        "SELECT a AS alias1, b AS alias2",
        // FROM / WHERE
        "SELECT * FROM users",
        "SELECT \"select\", name FROM users WHERE age > 18",
        "SELECT id, name FROM users WHERE age > 18",
        "SELECT * FROM users WHERE name LIKE 'A%'",
        "SELECT * FROM users WHERE created_at BETWEEN DATE '2020-01-01' AND DATE '2020-12-31'",
        "SELECT * FROM users WHERE status IS NOT NULL",
        // NOT predicates
        "SELECT * FROM t WHERE x NOT BETWEEN 1 AND 10",
        "SELECT * FROM t WHERE x NOT LIKE '%foo%'",
        "SELECT * FROM t WHERE x NOT IN (1, 2, 3)",
        "SELECT * FROM t WHERE x NOT IN (SELECT id FROM other)",
        // EXISTS
        "SELECT * FROM users u WHERE EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id)",
        "SELECT * FROM users u WHERE NOT EXISTS (SELECT 1 FROM bans b WHERE b.user_id = u.id)",
        // OVERLAPS (using explicit ROW constructors)
        "SELECT * FROM events WHERE ROW(start_date, end_date) OVERLAPS ROW(DATE '2024-01-01', DATE '2024-12-31')",
        // Joins
        "SELECT u.id, o.id FROM users u INNER JOIN orders o ON u.id = o.user_id",
        "SELECT * FROM a LEFT JOIN b ON a.id = b.a_id",
        "SELECT * FROM a RIGHT JOIN b ON a.id = b.a_id",
        "SELECT * FROM a FULL JOIN b ON a.id = b.a_id",
        "SELECT * FROM t1 CROSS JOIN t2",
        "SELECT * FROM a NATURAL JOIN b",
        "SELECT * FROM a INNER JOIN b USING (id)",
        // Multiple joins
        "SELECT * FROM a INNER JOIN b ON a.id = b.a_id INNER JOIN c ON b.id = c.b_id",
        // Grouping / Having
        "SELECT age, COUNT(*) FROM users GROUP BY age",
        "SELECT dept, AVG(salary) FROM employees GROUP BY dept HAVING AVG(salary) > 50000",
        // Ordering with NULLS
        "SELECT * FROM users ORDER BY name ASC",
        "SELECT * FROM users ORDER BY age DESC, name ASC",
        "SELECT * FROM users ORDER BY score DESC NULLS LAST",
        "SELECT * FROM users ORDER BY name ASC NULLS FIRST",
        // LIMIT / OFFSET / FETCH
        "SELECT * FROM users LIMIT 10",
        "SELECT * FROM users LIMIT 10 OFFSET 20",
        "SELECT * FROM users OFFSET 20 ROWS FETCH FIRST 10 ROWS ONLY",
        // Window functions
        "SELECT id, RANK() OVER (PARTITION BY dept ORDER BY salary DESC) AS r FROM employees",
        "SELECT id, SUM(amount) OVER (ORDER BY created_at ROWS BETWEEN 1 PRECEDING AND CURRENT ROW) FROM orders",
        "SELECT id, AVG(value) OVER (ORDER BY ts RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM data",
        "SELECT id, LAG(value) OVER w, LEAD(value) OVER w FROM t WINDOW w AS (ORDER BY ts)",
        // Subqueries
        "SELECT * FROM (SELECT id, name FROM users) u",
        "SELECT id, (SELECT COUNT(*) FROM orders o WHERE o.user_id = u.id) AS order_count FROM users u",
        "SELECT * FROM users WHERE id IN (SELECT user_id FROM orders)",
        "SELECT * FROM users WHERE salary > (SELECT AVG(salary) FROM users)",
        // Quantified comparisons (subqueries displayed with extra parens)
        "SELECT * FROM t WHERE x = ANY((SELECT y FROM other))",
        "SELECT * FROM t WHERE x > ALL((SELECT y FROM other))",
        // Set operations
        "SELECT id FROM users UNION SELECT id FROM admins",
        "SELECT id FROM users UNION ALL SELECT id FROM admins",
        "SELECT id FROM a INTERSECT SELECT id FROM b",
        "SELECT id FROM a EXCEPT SELECT id FROM b",
        // CTEs
        "WITH cte AS (SELECT id FROM users) SELECT * FROM cte",
        "WITH RECURSIVE tree AS (SELECT id, parent_id FROM nodes WHERE parent_id IS NULL UNION ALL SELECT n.id, n.parent_id FROM nodes n INNER JOIN tree t ON n.parent_id = t.id) SELECT * FROM tree",
        "WITH a AS (SELECT 1 AS x), b AS (SELECT 2 AS y) SELECT * FROM a, b",
        // Advanced GROUP BY
        "SELECT a, b, SUM(c) FROM t GROUP BY ROLLUP(a, b)",
        "SELECT a, b, SUM(c) FROM t GROUP BY CUBE(a, b)",
        "SELECT a, b, SUM(c) FROM t GROUP BY GROUPING SETS((a, b), (a), (b), ())",
        "SELECT a, b, c, SUM(d) FROM t GROUP BY a, ROLLUP(b, c)",
        // DML: INSERT
        "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')",
        "INSERT INTO users (id, name) VALUES (1, 'Alice'), (2, 'Bob')",
        "INSERT INTO logs SELECT * FROM temp_logs",
        "INSERT INTO counts (n) SELECT COUNT(*) FROM items",
        // DML: UPDATE
        "UPDATE users SET name = 'Bob' WHERE id = 1",
        "UPDATE users SET name = 'Bob', email = 'bob@example.com' WHERE id = 1",
        "UPDATE users SET age = age + 1",
        // DML: DELETE
        "DELETE FROM users WHERE id = 1",
        "DELETE FROM logs WHERE created_at < DATE '2020-01-01'",
        // VALUES as statement / in CTEs
        "VALUES (1, 'a'), (2, 'b'), (3, 'c')",
        "WITH v AS (VALUES (1), (2), (3)) SELECT * FROM v",
    ];
    assert_display_match(&inputs, ANSI_COMPLIANT_SPECS);
}

#[test]
fn ansi_partial() {
    let matching = [
        "SELECT * FROM ",
        "SELECT a FROM t WHERE ",
        "SELECT a FROM t WHERE a = ",
        "SELECT  FROM t",
        "SELECT a FROM t WHERE a BETWEEN ",
    ];
    assert_display_match(&matching, ANSI_COMPLIANT_SPECS);

    let inputs = [
        ("SELECT a. FROM t ", "SELECT a FROM t"),
        ("SELECT a, FROM t ", "SELECT a FROM t"),
        // Subqueries & set ops
        ("SELECT * FROM (SELECT", "SELECT * FROM (SELECT )"),
    ];
    assert_display_match_pairs(&inputs, ANSI_COMPLIANT_SPECS);
}

#[test]
fn pg_complete() {
    let inputs = [
        // DISTINCT ON (Postgres extension)
        "SELECT DISTINCT ON (user_id) user_id, created_at FROM events ORDER BY user_id, created_at DESC",
        // ILIKE, regex, SIMILAR TO + ESCAPE
        "SELECT * FROM users WHERE name ILIKE 'al%'",
        "SELECT * FROM users WHERE name NOT ILIKE 'test%'",
        "SELECT * FROM users WHERE email ~ '^[A-Za-z0-9._%+-]+@example\\.com$'",
        "SELECT * FROM users WHERE email !~ 'spam'",
        "SELECT * FROM users WHERE email ~* 'EXAMPLE'",
        "SELECT * FROM users WHERE email !~* 'SPAM'",
        "SELECT * FROM users WHERE note SIMILAR TO '(foo|bar)%' ESCAPE '\\'",
        // :: type cast (Postgres-specific) - displays as CAST()
        "SELECT CAST(x AS integer) FROM t",
        "SELECT CAST(CAST(x AS text) AS integer) FROM t",
        "SELECT CAST((a + b) AS numeric(10, 2)) FROM t",
        "SELECT CAST('2024-01-01' AS date)",
        "SELECT CAST(data AS jsonb)->>'name' FROM t",
        // Array subscript
        "SELECT arr[1] FROM t",
        "SELECT arr[1:3] FROM t",
        "SELECT matrix[1][2] FROM t",
        "SELECT (ARRAY[1,2,3])[2]",
        // AT TIME ZONE
        "SELECT ts AT TIME ZONE 'UTC' FROM events",
        "SELECT created_at AT TIME ZONE 'America/New_York' FROM users",
        "SELECT TIMESTAMP '2024-01-01 12:00:00' AT TIME ZONE 'UTC'",
        // JSONB operators
        "SELECT data->'profile' AS profile FROM accounts",
        "SELECT data->>'email' AS email FROM accounts",
        "SELECT data#>'{address,city}' AS city FROM accounts",
        "SELECT data#>>'{address,city}' AS city_text FROM accounts",
        "SELECT * FROM accounts WHERE data ? 'email'",
        "SELECT * FROM accounts WHERE data ?| ARRAY['email','phone']",
        "SELECT * FROM accounts WHERE data ?& ARRAY['email','phone']",
        "SELECT * FROM accounts WHERE data @? '$.email'",
        "SELECT * FROM accounts WHERE data @@ '$.active == true'",
        // Arrays (ANY/SOME/ALL and overlap/containment)
        "SELECT * FROM posts WHERE 'pg' = ANY(tags)",
        "SELECT * FROM posts WHERE 'pg' = SOME(tags)",
        "SELECT * FROM posts WHERE x > ALL(ARRAY[1,2,3])",
        "SELECT * FROM posts WHERE tags && ARRAY['pg','db']",
        "SELECT * FROM posts WHERE ARRAY['pg','db'] <@ tags",
        "SELECT * FROM posts WHERE tags @> ARRAY['pg']",
        // Bitwise operators
        "SELECT a & b, a | b, a # b FROM t",
        "SELECT a << 2, b >> 1 FROM t",
        // DISTINCT ON + LIMIT/OFFSET combo
        "SELECT DISTINCT ON (user_id) * FROM messages ORDER BY user_id, created_at DESC LIMIT 10 OFFSET 20",
        // LATERAL
        "SELECT u.id, x.word FROM users u, LATERAL unnest(string_to_array(u.bio,' ')) AS x(word)",
        "SELECT u.id, j.key, j.value FROM users u LEFT JOIN LATERAL jsonb_each(u.data) AS j(key,value) ON TRUE",
        // WINDOW name + FILTER
        "SELECT dept, SUM(salary) FILTER (WHERE active) OVER w AS active_sum, SUM(salary) OVER w AS total_sum FROM emp WINDOW w AS (PARTITION BY dept ORDER BY hired_at)",
        // Complex expressions (note: display uses no spaces after commas)
        "SELECT COALESCE(a,b,c) FROM t",
        "SELECT NULLIF(a,0) FROM t",
        "SELECT CASE WHEN CAST(x AS integer) > 0 THEN 'positive' ELSE 'negative' END FROM t",
    ];
    assert_display_match(&inputs, &[&postgres::SPEC]);
}

// ---------------- Test utils ----------------
fn statement(sql: &str, s: &DialectSpec) -> ast::Statement {
    let tokens = lex(s, sql);
    let mut parser = WinnowParser::new(&tokens, s);
    parser.parse_statement().unwrap().item
}

fn assert_display_match(inputs: &[&str], specs: &[&DialectSpec]) {
    for spec in specs {
        for input in inputs {
            let s = statement(input, spec);
            let output = fmt(input, &s);
            if output != *input {
                eprintln!("Input:  {:?}", input);
                eprintln!("Output: {:?}", output);
                eprintln!("AST:    {:#?}", s);
            }
            assert_eq!(input.to_string(), output);
        }
    }
}

fn assert_display_match_pairs(inputs: &[(&str, &str)], specs: &[&DialectSpec]) {
    for spec in specs {
        for (input, expected) in inputs {
            let s = statement(input, spec);
            let output = fmt(input, &s);
            if output != *expected {
                eprintln!("Input:  {:?}", input);
                eprintln!("Output: {:?}", output);
                eprintln!("AST:    {:#?}", s);
            }
            assert_eq!(output, expected.to_string());
        }
    }
}

const ANSI_COMPLIANT_SPECS: &[&'static DialectSpec] =
    &[&ansi::SPEC, &postgres::SPEC, &sqlite::SPEC];

fn fmt<T: AstDisplay>(input: &str, display: &T) -> String {
    display.display(input)
}
