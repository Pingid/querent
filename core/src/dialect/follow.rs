use crate::lex::{Keyword, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleSet(pub &'static [Rule]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule(pub If, pub &'static [Then]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Then {
    Kw(Keyword),
    CombinedKw(&'static [Keyword]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum If {
    Start,

    AnyKw,
    Kw(Keyword),
    Kind(TokenKind),

    AnyOf(&'static [If]),
    Not(&'static If),
    While(&'static If),
    Match(&'static [If]),
    /// Consume until the given if matches
    Until(&'static If),
}

impl If {
    pub fn match_consume(&self, tokens: &[TokenKind], offset: usize) -> (bool, usize) {
        match self {
            If::Start => match tokens.len() {
                0 => (true, 0),
                1 => tokens
                    .last()
                    .map_or((false, 0), |t| (t == &TokenKind::Identifier, 0)),
                _ => (false, 0),
            },

            If::AnyKw => match tokens.get(offset) {
                Some(TokenKind::Keyword(_)) => (true, offset.saturating_sub(1)),
                _ => (false, offset.saturating_sub(1)),
            },

            If::Kw(kw) => match tokens.get(offset) {
                Some(t) if t == &TokenKind::Keyword(*kw) => (true, offset.saturating_sub(1)),
                _ => (false, offset.saturating_sub(1)),
            },

            If::Kind(kind) => match tokens.get(offset) {
                Some(t) if t == kind => (true, offset.saturating_sub(1)),
                _ => (false, offset.saturating_sub(1)),
            },

            If::AnyOf(ifs) => ifs
                .iter()
                .find_map(|if_| {
                    let (m, o) = if_.match_consume(tokens, offset);
                    if m { Some((true, o)) } else { None }
                })
                .unwrap_or((false, offset)),

            If::Not(if_) => {
                let (m, _) = if_.match_consume(tokens, offset);
                match m {
                    // inner matched -> negation fails, no consumption
                    true => (false, offset),
                    // inner didn't match -> negation succeeds, consume current token
                    false => (true, offset.saturating_sub(1)),
                }
            }

            If::While(if_) => {
                let mut o = offset;
                loop {
                    let (m, next) = if_.match_consume(tokens, o);
                    if !m {
                        break;
                    }
                    if next == o {
                        break;
                    }
                    o = next;
                }
                (true, o)
            }

            If::Match(ifs) => {
                let mut o = offset;
                for if_ in ifs.iter().rev() {
                    let (m, next) = if_.match_consume(tokens, o);
                    if !m {
                        return (false, o);
                    }
                    o = next;
                }
                (true, o)
            }

            If::Until(if_) => {
                let mut o = offset;
                let mut iterations = 0;
                loop {
                    iterations += 1;
                    if iterations > 100 {
                        eprintln!("WARNING: If::Until exceeded 100 iterations");
                        eprintln!("  offset: {}", offset);
                        eprintln!("  current o: {}", o);
                        eprintln!("  tokens: {:?}", tokens);
                        eprintln!("  condition: {:?}", if_);
                        return (false, o);
                    }
                    // Check if the condition matches at current position
                    let (m, _) = if_.match_consume(tokens, o);
                    if m {
                        // Found the match, return with current offset
                        return (true, o);
                    }
                    // No more tokens to consume
                    if o == 0 {
                        return (false, 0);
                    }
                    // Move to next token
                    o = o.saturating_sub(1);
                }
            }
        }
    }
}

pub fn resolve_follow_rules(
    rules: &[RuleSet],
    tokens: &[TokenKind],
) -> impl Iterator<Item = String> {
    let t = match tokens.last() {
        Some(t) if t == &TokenKind::Eof => &tokens[..tokens.len().saturating_sub(1)],
        _ => tokens,
    };
    rules
        .iter()
        .flat_map(|r| get_matches(r.0, t, t.len().saturating_sub(1)))
        .map(|r| format!("{}", r))
}

fn get_matches(rules: &[Rule], tokens: &[TokenKind], offset: usize) -> impl Iterator<Item = Then> {
    rules
        .iter()
        .filter(move |r| r.0.match_consume(tokens, offset).0)
        .map(|r| r.1.iter().copied())
        .flatten()
}

impl std::fmt::Display for Then {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Then::Kw(kw) => write!(f, "{}", format!("{:?}", kw).to_uppercase()),
            Then::CombinedKw(kws) => write!(
                f,
                "{}",
                kws.iter()
                    .map(|kw| format!("{:?}", kw).to_uppercase())
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::ansi_tokens;

    use super::*;

    #[test]
    fn if_start() {
        assert_matches(true, If::Start, "SELEC");
        assert_matches(true, If::Start, "");
        assert_matches(false, If::Start, "SELECT");
    }

    #[test]
    fn if_kw() {
        assert_matches(true, If::Kw(Keyword::Select), "SELECT");
        assert_matches(false, If::Kw(Keyword::Select), "SELECT DISTINCT");
    }

    #[test]
    fn if_kind() {
        use TokenKind::*;
        assert_matches(true, If::Kind(LeftParen), "(");
        assert_matches(false, If::Kind(LeftParen), ")");
    }

    #[test]
    fn if_match() {
        let rule = If::Match(&[
            If::Kw(Keyword::Select),
            If::While(&If::Not(&If::AnyOf(&[
                If::Kw(Keyword::From),
                If::Kw(Keyword::Select),
            ]))),
            If::AnyOf(&[
                If::Kind(TokenKind::Identifier),
                If::Kind(TokenKind::RightParen),
            ]),
        ]);
        assert_matches(true, rule, "SELECT a");
        assert_matches(false, rule, "SELECT ");
        assert_matches(false, rule, "SELECT id FROM");
        assert_matches(true, rule, "SELECT id, b, c, d");
    }

    #[test]
    fn if_match_limit() {
        // Match when we're after a query body but haven't seen LIMIT/UNION/etc yet
        // Using Until to scan backwards and ensure none of these keywords appear
        let rule = If::Match(&[
            If::Not(&If::Until(&If::AnyOf(&[
                If::Kw(Keyword::Limit),
                If::Kw(Keyword::Union),
                If::Kw(Keyword::Intersect),
                If::Kw(Keyword::Except),
                If::Kw(Keyword::Offset),
                If::Kw(Keyword::Fetch),
            ]))),
            If::AnyOf(&[
                If::Kind(TokenKind::Identifier),
                If::Kind(TokenKind::RightParen),
                If::Kind(TokenKind::Number),
                If::Kind(TokenKind::Str),
            ]),
        ]);
        // Basic FROM clause - LIMIT can appear after table name
        assert_matches(true, rule, "SELECT a FROM users ");
        // FROM with WHERE clause - LIMIT can appear after WHERE
        assert_matches(true, rule, "SELECT a FROM users WHERE name = 'John' ");
        // FROM with GROUP BY clause - LIMIT can appear after GROUP BY
        assert_matches(true, rule, "SELECT a FROM users GROUP BY id ");
        // FROM with HAVING clause - LIMIT can appear after HAVING
        assert_matches(
            true,
            rule,
            "SELECT a FROM users GROUP BY id HAVING count > 1 ",
        );
        // FROM with ORDER BY clause - LIMIT can appear after ORDER BY
        assert_matches(true, rule, "SELECT a FROM users ORDER BY name ");
        // Should not match after UNION (LIMIT typically goes at the end of combined query)
        assert_matches(
            false,
            rule,
            "SELECT a FROM users UNION SELECT b FROM others ",
        );
        // Should not match after LIMIT already exists
        assert_matches(false, rule, "SELECT a FROM users LIMIT ");
        assert_matches(false, rule, "SELECT a FROM users LIMIT 10");
        assert_matches(false, rule, "SELECT a FROM users OFFSET ");
        assert_matches(false, rule, "SELECT a FROM users OFFSET 5 ");
    }

    fn assert_matches(matches: bool, rule: If, sql: &str) {
        let tokens = ansi_tokens(sql);
        let kinds = tokens.iter().map(|t| t.kind).collect::<Vec<TokenKind>>();
        let kinds = &kinds[0..kinds.len().saturating_sub(1)]; // ignore the last token (EOF)
        assert_eq!(
            rule.match_consume(kinds, kinds.len().saturating_sub(1)).0,
            matches,
            "\nrule: {:?}\ntokens: {:?}",
            rule,
            kinds
        );
    }
}
