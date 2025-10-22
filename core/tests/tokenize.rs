use proptest::prelude::*;
use querent_core::dialect::ansi;
use querent_core::lex::{Keyword, Token, TokenKind, lex};

#[test]
fn basic_select_line() {
    let input = "SELECT name, * FROM users WHERE age > 18 AND name = 'John'";
    let s = &ansi::SPEC;
    let toks = lex(s, input);
    let expected = [
        TokenKind::Keyword(Keyword::Select),
        TokenKind::Identifier,
        TokenKind::Comma,
        TokenKind::Operator(s.match_operator("*").unwrap()),
        TokenKind::Keyword(Keyword::From),
        TokenKind::Identifier,
        TokenKind::Keyword(Keyword::Where),
        TokenKind::Identifier,
        TokenKind::Operator(s.match_operator(">").unwrap()),
        TokenKind::Number,
        TokenKind::Operator(s.match_operator("AND").unwrap()),
        TokenKind::Identifier,
        TokenKind::Operator(s.match_operator("=").unwrap()),
        TokenKind::Str,
        TokenKind::Eof,
    ];
    for (i, tok) in toks.iter().enumerate() {
        assert_eq!(Some(tok.kind), expected.get(i).copied(), "index {i}");
    }
}

#[test]
fn comments_and_ops_are_skipped_and_recognized() {
    let toks = ansi_tokens("/* a */ SELECT -- line\n a<=1 AND b!=2");
    assert!(
        toks.iter()
            .any(|t| t.kind == TokenKind::Keyword(Keyword::Select))
    );
    assert!(
        toks.iter()
            .any(|t| matches!(t.kind, TokenKind::Operator(_)))
    );
}

#[test]
fn word_operator_greediness_identifier_not_split() {
    let toks = ansi_tokens("ANDY");
    assert_eq!(toks.len(), 2);
    assert_eq!(toks[0].kind, TokenKind::Identifier);
    assert_eq!(toks[0].text, "ANDY");
    assert_eq!(toks[1].kind, TokenKind::Eof);

    // Similar check for LIKE
    let toks2 = ansi_tokens("LIKELY");
    assert_eq!(toks2[0].kind, TokenKind::Identifier);
    assert_eq!(toks2[0].text, "LIKELY");
}

#[test]
fn case_insensitive_keywords() {
    let kinds = ansi_tokens("select a from t")
        .iter()
        .map(|t| t.kind)
        .collect::<Vec<_>>();
    assert_eq!(kinds[0], TokenKind::Keyword(Keyword::Select));
    assert_eq!(kinds[2], TokenKind::Keyword(Keyword::From));
}

#[test]
fn punctuation_and_grouping_tokens() {
    let toks = ansi_token_kinds("(a,b);");
    assert_eq!(
        toks,
        vec![
            TokenKind::LeftParen,
            TokenKind::Identifier,
            TokenKind::Comma,
            TokenKind::Identifier,
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::Eof
        ]
    );
}

#[test]
fn string_with_escaped_quote() {
    let toks = ansi_tokens("WHERE x = 'a''b'");
    assert!(toks.iter().any(|t| t.kind == TokenKind::Str));
}

#[test]
fn floats_and_numbers_cover_edge_forms() {
    // Leading dot
    let toks = ansi_tokens(".5");
    assert_eq!(toks[0].kind, TokenKind::Float);
    assert_eq!(toks[0].text, ".5");

    // Trailing dot (commonly accepted)
    let toks2 = ansi_tokens("1.");
    assert_eq!(toks2[0].kind, TokenKind::Float);
    assert_eq!(toks2[0].text, "1.");

    // Fraction with exponent after dot
    let toks3 = ansi_tokens("1.e10");
    assert_eq!(toks3[0].kind, TokenKind::Float);
    assert_eq!(toks3[0].text, "1.e10");
}

#[test]
fn nested_block_comments_are_consumed() {
    let toks = ansi_tokens("/* outer /* inner */ */ SELECT 1");
    assert_eq!(toks.len(), 3);
    assert_eq!(toks[0].kind, TokenKind::Keyword(Keyword::Select));
    assert_eq!(toks[1].kind, TokenKind::Number);
    assert_eq!(toks[2].kind, TokenKind::Eof);
}

// ---- Property tests ----

proptest! {
    // Never panic on arbitrary input
    #[test]
    fn never_panics(input in "\\PC*") {
        let _ = ansi_tokens(&input);
    }

    // Always terminates with Eof and consumes full input
    #[test]
    fn terminates_and_consumes_all(input in "\\PC{0,200}") {
        let tokens = ansi_tokens(&input);
        prop_assert!(!tokens.is_empty(), "no tokens produced");
        prop_assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
        prop_assert_eq!(tokens.last().unwrap().span.end, input.len());
    }

    // Spans are monotonic and non-overlapping
    #[test]
    fn spans_monotonic_and_non_overlapping(input in "\\PC{0,200}") {
        let tokens = ansi_tokens(&input);
        for w in tokens.windows(2) {
            let a = &w[0]; let b = &w[1];
            prop_assert!(a.span.start <= a.span.end);
            prop_assert!(a.span.end <= b.span.start);
            prop_assert!(b.span.start <= b.span.end);
        }
    }

    // If a token has non-empty text, cursor must advance
    #[test]
    fn nonempty_tokens_advance(input in "\\PC{0,200}") {
        let tokens = ansi_tokens(&input);
        let mut last_end = 0usize;
        for t in tokens {
            if !t.text.is_empty() {
                prop_assert!(t.span.end > last_end, "cursor did not advance");
            }
            last_end = t.span.end;
        }
    }

    // Random numeric forms look like numbers/floats at least once
    #[test]
    fn random_numbers_detected(n in 0u64..1_000_000, f in 0.0f64..1_000_000.0) {
        let n = n.to_string();
        let int_toks = ansi_tokens(&n);
        prop_assert!(int_toks.iter().any(|t| t.kind == TokenKind::Number));

        let float_str = format!("{:.6}", f);
        let float_toks = ansi_tokens(&float_str);
        prop_assert!(float_toks.iter().any(|t| t.kind == TokenKind::Float));
    }
}

pub fn ansi_tokens<'a>(input: &'a str) -> Vec<Token<'a>> {
    let s = &ansi::SPEC;
    lex(s, input)
}

pub fn ansi_token_kinds(input: &str) -> Vec<TokenKind> {
    let s = &ansi::SPEC;
    lex(s, input)
        .into_iter()
        .map(|t| t.kind)
        .collect::<Vec<_>>()
}
