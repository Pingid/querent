//! Compares the three SQL parser implementations on a fixed in-memory corpus:
//! - `recursive_descent`: the original hand-written parser (`parse::Parser`).
//! - `handrolled_v2`: the hand-rolled combinator parser (`parse::v2`).
//! - `winnow_lib`: the `winnow`-library parser (`parse_new`).
//!
//! Tokens are lexed once up front so the benchmark isolates parsing cost.
//! Run with: `cargo bench -p querent-core --features bench`.

use std::hint::black_box;
use std::time::Duration;

use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use querent_core::dialect::postgres;
use querent_core::lex::Token;
use querent_core::lex::TokenKind;
use querent_core::lex::lex;
use querent_core::parse::Parser;
use querent_core::parse::v2::ParserV2;
use querent_core::parse_new;

/// Load SQL text from `QUERENT_BENCH_FILE`, if set, stripping the uploaded-file
/// header lines and the `use tpcds;` directive.
/// "https://raw.githubusercontent.com/memsql/benchmarks-tpc/refs/heads/master/tpcds/queries.sql";
fn load_corpus_text() -> Option<String> {
    let path = std::env::var("QUERENT_BENCH_FILE").ok()?;
    let raw = std::fs::read_to_string(&path).ok()?;
    let cleaned = raw
        .lines()
        .filter(|l| !l.starts_with("Source URL:") && !l.starts_with("Title:"))
        .collect::<Vec<_>>()
        .join("\n");
    Some(cleaned.replace("use tpcds;", ""))
}

/// Split `input` into individual statements on top-level semicolons.
fn split_statements(input: &str) -> Vec<&str> {
    let tokens = lex(&postgres::SPEC, input);
    let mut bounds: Vec<(usize, usize)> = Vec::new();
    let mut start = 0usize;
    for t in &tokens {
        match t.kind {
            TokenKind::Semicolon => {
                bounds.push((start, t.span.end));
                start = t.span.end;
            }
            TokenKind::Eof => bounds.push((start, t.span.start)),
            _ => {}
        }
    }
    bounds
        .into_iter()
        .filter_map(|(lo, hi)| {
            let s = &input[lo..hi];
            let trimmed = s.trim();
            (!trimmed.is_empty()).then(|| {
                let off = s.find(trimmed).unwrap_or(0);
                &input[lo + off..lo + off + trimmed.len()]
            })
        })
        .collect()
}

/// Representative SQL spanning the parser's feature surface.
fn corpus() -> Vec<&'static str> {
    vec![
        "SELECT 1",
        "SELECT id, name, email FROM users",
        "SELECT * FROM users WHERE id = 1 AND name = 'foo'",
        "SELECT u.id, p.title FROM users u JOIN posts p ON u.id = p.user_id",
        "SELECT a, b, c FROM t LEFT OUTER JOIN s ON t.k = s.k WHERE a > 1 ORDER BY b DESC LIMIT 10",
        "WITH cte AS (SELECT id FROM users WHERE active) SELECT * FROM cte JOIN posts ON cte.id = posts.user_id",
        "SELECT count(*) FILTER (WHERE x > 0), sum(y) FROM t GROUP BY z HAVING count(*) > 5",
        "SELECT rank() OVER (PARTITION BY dept ORDER BY salary DESC) FROM emp",
        "SELECT CASE WHEN x > 1 THEN 'a' WHEN x > 0 THEN 'b' ELSE 'c' END FROM t",
        "SELECT x::numeric(10,2), y::text, arr[1:3] FROM t WHERE z BETWEEN 1 AND 100",
        "SELECT * FROM a, b, c WHERE a.id = b.id AND b.id = c.id AND a.val IN (1, 2, 3, 4, 5)",
        "SELECT * FROM t WHERE name LIKE '%foo%' AND created_at NOT BETWEEN '2020' AND '2021'",
        "SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 EXCEPT SELECT 4",
        "INSERT INTO t (a, b, c) VALUES (1, 2, 3), (4, 5, 6) RETURNING id",
        "UPDATE t SET a = 1, b = a + 2, c = (SELECT max(x) FROM s) WHERE id = 1 RETURNING *",
        "DELETE FROM t USING s WHERE t.id = s.id AND s.flag = true",
        "SELECT a, b, SUM(c) FROM t GROUP BY GROUPING SETS ((a, b), (a), ())",
        "SELECT * FROM (SELECT id, ROW_NUMBER() OVER (ORDER BY ts) rn FROM events) e WHERE e.rn = 1",
        "SELECT EXISTS (SELECT 1 FROM s WHERE s.id = t.id), t.* FROM t",
        "SELECT (a + b) * c - d / e % f, -g, NOT h FROM t WHERE i IS NOT NULL AND j IS NULL",
    ]
}

fn bench_parsers(c: &mut Criterion) {
    let spec = &postgres::SPEC;
    let file_text = load_corpus_text();
    let queries: Vec<&str> = match &file_text {
        Some(text) => split_statements(text),
        None => corpus(),
    };
    let tokenized: Vec<Vec<Token>> = queries.iter().map(|q| lex(spec, q)).collect();

    // One-time coverage report: how many statements each parser fully accepts.
    let ok_rd = tokenized
        .iter()
        .filter(|t| Parser::new(*t).parse_statement().is_some())
        .count();
    let ok_v2 = tokenized
        .iter()
        .filter(|t| ParserV2::new(t, spec).parse_statement().is_some())
        .count();
    let ok_new = tokenized
        .iter()
        .filter(|t| parse_new::parse_statement(t, spec).is_some())
        .count();
    eprintln!(
        "corpus: {} statements | parsed Some -> recursive_descent={} handrolled_v2={} winnow_lib={}",
        queries.len(),
        ok_rd,
        ok_v2,
        ok_new,
    );

    let mut group = c.benchmark_group("parse");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(60);
    group.throughput(Throughput::Elements(queries.len() as u64));

    group.bench_function("recursive_descent", |b| {
        b.iter(|| {
            for toks in &tokenized {
                let mut p = Parser::new(toks);
                black_box(p.parse_statement());
            }
        })
    });

    group.bench_function("handrolled_v2", |b| {
        b.iter(|| {
            for toks in &tokenized {
                let mut p = ParserV2::new(toks, spec);
                black_box(p.parse_statement());
            }
        })
    });

    group.bench_function("winnow_lib", |b| {
        b.iter(|| {
            for toks in &tokenized {
                black_box(parse_new::parse_statement(toks, spec));
            }
        })
    });

    group.finish();
}

criterion_group!(benches, bench_parsers);
criterion_main!(benches);
