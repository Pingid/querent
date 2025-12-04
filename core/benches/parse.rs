use std::time::Duration;

use criterion::BatchSize;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use querent_core::dialect::postgres;
use querent_core::lex::TokenKind;
use querent_core::lex::lex;
use querent_core::parse::Parser;
use querent_core::parse::v2::WinnowParser;

const DEFAULT_URL: &str =
    "https://raw.githubusercontent.com/memsql/benchmarks-tpc/refs/heads/master/tpcds/queries.sql";

fn fetch_queries_text() -> String {
    let url = std::env::var("QUERENT_BENCH_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
    match reqwest::blocking::get(&url) {
        Ok(resp) => match resp.text() {
            Ok(text) => text,
            Err(err) => panic!("failed to read response text from {url}: {err}"),
        },
        Err(err) => panic!("failed to fetch {url}: {err}. Set QUERENT_BENCH_URL to override."),
    }
}

fn split_statements_pg(input: &str) -> Vec<&str> {
    let spec = &postgres::SPEC;
    let tokens = lex(spec, input);

    let mut stmts = Vec::new();
    let mut start = 0usize;
    for t in &tokens {
        match t.kind {
            TokenKind::Semicolon => {
                let end = t.span.end;
                if end > start {
                    // Trim leading/trailing whitespace to avoid empty statements
                    let s = &input[start..end];
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        // Map trimmed back to slice of original input, if possible
                        let offset_front = s.find(trimmed).unwrap_or(0);
                        let slice =
                            &input[start + offset_front..start + offset_front + trimmed.len()];
                        stmts.push(slice);
                    }
                }
                start = end; // after semicolon
            }
            TokenKind::Eof => {
                let end = t.span.start; // Eof at cursor
                if end > start {
                    let s = &input[start..end];
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        let offset_front = s.find(trimmed).unwrap_or(0);
                        let slice =
                            &input[start + offset_front..start + offset_front + trimmed.len()];
                        stmts.push(slice);
                    }
                }
            }
            _ => {}
        }
    }
    stmts
}

fn bench_tpcds(c: &mut Criterion) {
    // Fetch queries
    let text = fetch_queries_text().replace("use tpcds;", "");
    let stmts = split_statements_pg(&text);

    // Expose counts for throughput
    let total_bytes: usize = stmts.iter().map(|s| s.len()).sum();
    let total_queries = stmts.len() as u64;

    let mut group = c.benchmark_group("tpcds");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(30);

    // Tokenize all queries as a single iteration
    // group.throughput(Throughput::Bytes(total_bytes as u64));
    // group.bench_function(BenchmarkId::new("tokenize_all", "pg"), |b| {
    //     b.iter(|| {
    //         let spec = &postgres::SPEC;
    //         for q in &stmts {
    //             let _ = lex(spec, q);
    //         }
    //     });
    // });

    // Parse all queries as a single iteration
    group.throughput(Throughput::Elements(total_queries));
    // group.bench_function(BenchmarkId::new("parse_all", "pg"), |b| {
    //     b.iter(|| {
    //         let spec = &postgres::SPEC;
    //         for q in &stmts {
    //             let tokens = lex(spec, q);
    //             let mut parser = Parser::new(&tokens);
    //             let _ = parser.parse_statement();
    //         }
    //     });
    // });

    // // Per-query parsing (batched) for distribution insight
    // group.bench_function(BenchmarkId::new("parse_each", "pg"), |b| {
    //     b.iter_batched(
    //         || stmts.clone(),
    //         |cases| {
    //             let spec = &postgres::SPEC;
    //             for q in cases {
    //                 let tokens = lex(spec, q);
    //                 let mut parser = Parser::new(&tokens);
    //                 let _ = parser.parse_statement();
    //             }
    //         },
    //         BatchSize::SmallInput,
    //     )
    // });

    group.bench_function(BenchmarkId::new("parse_all_v2", "pg"), |b| {
        b.iter(|| {
            let spec = &postgres::SPEC;
            for q in &stmts {
                let tokens = lex(spec, q);
                let mut parser = WinnowParser::new(&tokens, spec);
                let _ = parser.parse_statement();
            }
        });
    });

    // // Per-query parsing (batched) for distribution insight
    // group.bench_function(BenchmarkId::new("parse_each_v2", "pg"), |b| {
    //     b.iter_batched(
    //         || stmts.clone(),
    //         |cases| {
    //             let spec = &postgres::SPEC;
    //             for q in cases {
    //                 let tokens = lex(spec, q);
    //                 let mut parser = WinnowParser::new(&tokens, spec);
    //                 let _ = parser.parse_statement();
    //             }
    //         },
    //         BatchSize::SmallInput,
    //     )
    // });

    group.finish();
}

criterion_group!(benches, bench_tpcds);
criterion_main!(benches);
