use querent_core::{
    ast,
    dialect::{Ansi, Dialect, DialectSpec, Postgres},
    parse::Parser,
    token::lex,
};

mod common;
use common::ast::AstDisplay;

// ---------------- Test: Statement ----------------
#[test]
fn statement_partial_and_query() {
    let dialect = Ansi;
    let spec = dialect.get_spec();
    let ast = statement("", spec);
    assert!(matches!(ast, ast::Statement::Partial(_)));
    let ast = statement("SEL", spec);
    assert!(matches!(ast, ast::Statement::Partial(_)));
    let ast = statement("SELECT", spec);
    assert!(matches!(ast, ast::Statement::Query(_)));
}

#[test]
fn ans_complete() {
    // panic!("{:#?}", statement("SELECT *", &ansi_compliant_specs()[0]));
    let inputs = [
        // Literals & expressions
        "SELECT 1",
        "SELECT NULL",
        "SELECT a + b, c * d, e / f, g - h",
        "SELECT (a + b) * c",
        "SELECT CASE WHEN age > 18 THEN 'adult' ELSE 'child' END AS category",
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
        // Joins
        "SELECT u.id, o.id FROM users u INNER JOIN orders o ON u.id = o.user_id",
        "SELECT * FROM a LEFT JOIN b ON a.id = b.a_id",
        "SELECT * FROM a RIGHT JOIN b ON a.id = b.a_id",
        "SELECT * FROM a FULL JOIN b ON a.id = b.a_id",
        "SELECT * FROM t1 CROSS JOIN t2",
        // Grouping / Having
        "SELECT age, COUNT(*) FROM users GROUP BY age",
        "SELECT dept, AVG(salary) FROM employees GROUP BY dept HAVING AVG(salary) > 50000",
        // Ordering
        "SELECT * FROM users ORDER BY name ASC",
        "SELECT * FROM users ORDER BY age DESC, name ASC",
        "SELECT * FROM users OFFSET 20 ROWS FETCH FIRST 10 ROWS ONLY",
        "SELECT id, RANK() OVER (PARTITION BY dept ORDER BY salary DESC) AS r FROM employees",
        "SELECT id, SUM(amount) OVER (ORDER BY created_at ROWS BETWEEN 1 PRECEDING AND CURRENT ROW) FROM orders",
        // Subqueries
        "SELECT * FROM (SELECT id, name FROM users) u",
        "SELECT id, (SELECT COUNT(*) FROM orders o WHERE o.user_id = u.id) AS order_count FROM users u",
        "SELECT * FROM users WHERE id IN (SELECT user_id FROM orders)",
        // Set operations
        "SELECT id FROM users UNION SELECT id FROM admins",
        "SELECT id FROM a INTERSECT SELECT id FROM b",
        "SELECT id FROM a EXCEPT SELECT id FROM b",
        // CTEs
        "WITH cte AS (SELECT id FROM users) SELECT * FROM cte",
    ];
    assert_display_match(&inputs, &ansi_compliant_specs());
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
    assert_display_match(&matching, &ansi_compliant_specs());

    let inputs = [
        ("SELECT a. FROM t ", "SELECT a FROM t"),
        ("SELECT a, FROM t ", "SELECT a FROM t"),
        // Subqueries & set ops
        ("SELECT * FROM (SELECT", "SELECT * FROM (SELECT )"),
    ];
    assert_display_match_pairs(&inputs, &ansi_compliant_specs());
}

#[test]
fn pg_complete() {
    let inputs = [
        // DISTINCT ON (Postgres extension)
        "SELECT DISTINCT ON (user_id) user_id, created_at FROM events ORDER BY user_id, created_at DESC",
        // ILIKE, regex, SIMILAR TO + ESCAPE
        "SELECT * FROM users WHERE name ILIKE 'al%'",
        "SELECT * FROM users WHERE email ~ '^[A-Za-z0-9._%+-]+@example\\.com$'",
        "SELECT * FROM users WHERE note SIMILAR TO '(foo|bar)%' ESCAPE '\\'",
        // JSONB operators
        "SELECT data->'profile' AS profile FROM accounts",
        "SELECT data->>'email' AS email FROM accounts",
        "SELECT data#>'{address,city}' AS city FROM accounts",
        "SELECT data#>>'{address,city}' AS city_text FROM accounts",
        "SELECT * FROM accounts WHERE data ? 'email'",
        "SELECT * FROM accounts WHERE data ?| ARRAY['email','phone']",
        // Arrays (ANY/SOME/ALL and overlap/containment)
        "SELECT * FROM posts WHERE 'pg' = ANY(tags)",
        "SELECT * FROM posts WHERE tags && ARRAY['pg','db']",
        "SELECT * FROM posts WHERE ARRAY['pg','db'] <@ tags",
        "SELECT * FROM posts WHERE tags @> ARRAY['pg']",
        // DISTINCT ON + LIMIT/OFFSET combo
        "SELECT DISTINCT ON (user_id) * FROM messages ORDER BY user_id, created_at DESC LIMIT 10 OFFSET 20",
        // LATERAL
        "SELECT u.id, x.word FROM users u, LATERAL unnest(string_to_array(u.bio,' ')) AS x(word)",
        "SELECT u.id, j.key, j.value FROM users u LEFT JOIN LATERAL jsonb_each(u.data) AS j(key,value) ON TRUE",
        // WINDOW name + FILTER
        "SELECT dept, SUM(salary) FILTER (WHERE active) OVER w AS active_sum, SUM(salary) OVER w AS total_sum FROM emp WINDOW w AS (PARTITION BY dept ORDER BY hired_at)",
    ];
    let d = Postgres;
    let spec = d.get_spec();
    assert_display_match(&inputs, std::slice::from_ref(spec));
}

// ---------------- Test utils ----------------
fn statement(sql: &str, s: &DialectSpec) -> ast::Statement {
    let tokens = lex(s, sql);
    let mut parser = Parser::new(&tokens);
    parser.parse_statement().unwrap().item
}

fn assert_display_match(inputs: &[&str], specs: &[DialectSpec]) {
    for spec in specs {
        for input in inputs {
            let s = statement(input, spec);
            let output = fmt(input, &s);
            if output != *input {
                eprintln!("Input:  {:?}", input);
                eprintln!("Output: {:?}", output);
                eprintln!("AST:    {:#?}", s);
            }
            assert_eq!(output, input.to_string());
        }
    }
}

fn assert_display_match_pairs(inputs: &[(&str, &str)], specs: &[DialectSpec]) {
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

fn ansi_compliant_specs() -> Vec<DialectSpec> {
    let ansi = Ansi;
    let pg = Postgres;
    vec![ansi.get_spec().clone(), pg.get_spec().clone()]
}

fn fmt<T: AstDisplay>(input: &str, display: &T) -> String {
    display.display(input)
}
