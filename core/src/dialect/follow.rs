use std::collections::HashSet;

use crate::lex::Keyword;
use crate::lex::OpTag;
use crate::lex::Operator;
use crate::lex::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rules(pub &'static [Rule]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule(pub Cond, pub &'static [Next]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Next {
    Kw(Keyword),
    KwSeq(&'static [Keyword]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cond {
    /// Returns true when at the end in forward direction or when tokens are
    /// empty in backward direction.
    End,
    /// Consumes the token if it matches the given keyword.
    Kw(Keyword),
    /// Consumes the token if its operator tag matches.
    Op(OpTag),
    /// Consumes the token if it matches the given kind.
    Kind(TokenKind),
    /// Tries each condition and succeeds if any match. Takes the best (furthest
    /// in direction) match.
    Any(&'static [Cond]),
    /// Succeeds when the inner condition fails, consumes one token. Fails when
    /// inner succeeds.
    Not(&'static Cond),
    /// Continues consuming until the condition no longer holds. Always succeeds
    /// (even with zero matches).
    Many(&'static Cond),
    /// All conditions must match in order. Fails if any condition doesn't
    /// match.
    Seq(&'static [Cond]),
    /// Stops at (but doesn't consume) the first token where the condition
    /// holds. Fails if condition never matches.
    Until(&'static Cond),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    Fwd,
    Bwd,
}

impl Dir {
    fn step(&self, offset: usize, len: usize) -> usize {
        match self {
            Dir::Fwd => (offset.saturating_add(1)).min(len),
            Dir::Bwd => offset.saturating_sub(1),
        }
    }
    fn at_edge(&self, offset: usize, len: usize) -> bool {
        match self {
            Dir::Fwd => offset >= len,
            Dir::Bwd => offset == 0,
        }
    }
    fn get_offset(&self, offset: usize) -> usize {
        match self {
            Dir::Fwd => offset,
            Dir::Bwd => offset.saturating_sub(1),
        }
    }
}

impl Cond {
    pub fn scan(&self, ts: &[TokenKind], dir: Dir) -> (bool, usize) {
        match dir {
            Dir::Fwd => self.scan_from(ts, Dir::Fwd, 0),
            Dir::Bwd => self.scan_from(ts, Dir::Bwd, ts.len()),
        }
    }

    pub fn scan_from(&self, ts: &[TokenKind], dir: Dir, offset: usize) -> (bool, usize) {
        let mut fuel = ts.len().saturating_mul(64).max(256); // budget
        self.scan_inner(ts, offset, dir, &mut fuel)
    }

    fn scan_inner(
        &self, ts: &[TokenKind], offset: usize, direction: Dir, fuel: &mut usize,
    ) -> (bool, usize) {
        if *fuel == 0 {
            return (false, offset);
        }
        *fuel -= 1;

        match self {
            Cond::End => (direction.at_edge(offset, ts.len()), offset),

            Cond::Kw(kw) => match ts.get(direction.get_offset(offset)) {
                Some(t) if t == &TokenKind::Keyword(*kw) => {
                    (true, direction.step(offset, ts.len()))
                }
                _ => (false, offset), // <- don't move on mismatch
            },

            Cond::Op(op) => {
                match ts.get(direction.get_offset(offset)) {
                    Some(TokenKind::Operator(Operator { semantic_tag, .. }))
                        if semantic_tag == op =>
                    {
                        (true, direction.step(offset, ts.len()))
                    }
                    _ => (false, offset), // <- don't move
                }
            }

            Cond::Kind(kind) => match ts.get(direction.get_offset(offset)) {
                Some(t) if t == kind => (true, direction.step(offset, ts.len())),
                _ => (false, offset), // <- don't move
            },

            Cond::Any(ps) => {
                let mut best: Option<usize> = None;
                for p in *ps {
                    let (m, o2) = p.scan_inner(ts, offset, direction, fuel);
                    if !m {
                        continue;
                    }
                    best = Some(match (best, direction) {
                        (None, _) => o2,
                        (Some(o_best), Dir::Fwd) => o2.max(o_best),
                        (Some(o_best), Dir::Bwd) => o2.min(o_best),
                    });
                }
                match best {
                    Some(o) => (true, o),
                    None => (false, offset),
                }
            }

            Cond::Not(if_) => {
                let (m, _) = if_.scan_inner(ts, offset, direction, fuel);
                match m {
                    // inner matched -> negation fails, no consumption
                    true => (false, offset),
                    // inner didn't match -> negation succeeds, consume current token
                    false => (true, direction.step(offset, ts.len())),
                }
            }

            Cond::Many(if_) => {
                let mut o = offset;
                loop {
                    let (m, next) = if_.scan_inner(ts, o, direction, fuel);
                    if !m || next == o {
                        break;
                    }
                    o = next;
                }
                (true, o)
            }

            Cond::Seq(ifs) => {
                let mut o = offset;
                match direction {
                    Dir::Bwd => {
                        for if_ in ifs.iter().rev() {
                            let (m, next) = if_.scan_inner(ts, o, direction, fuel);
                            if !m {
                                return (false, o);
                            }
                            o = next;
                        }
                    }
                    Dir::Fwd => {
                        for if_ in ifs.iter() {
                            let (m, next) = if_.scan_inner(ts, o, direction, fuel);
                            if !m {
                                return (false, o);
                            }
                            o = next;
                        }
                    }
                }
                (true, o)
            }

            Cond::Until(if_) => {
                let mut o = offset;
                loop {
                    let (m, _) = if_.scan_inner(ts, o, direction, fuel);
                    if m {
                        return (true, o);
                    }
                    if direction.at_edge(o, ts.len()) {
                        return (false, o);
                    }
                    let next = direction.step(o, ts.len());
                    if next == o {
                        return (false, o);
                    }
                    o = next;
                }
            }
        }
    }
}

pub fn resolve_next<'a>(
    rules: &'a [Rules], tokens: &'a [TokenKind],
) -> impl Iterator<Item = Vec<Keyword>> + 'a {
    let t = match tokens.last() {
        Some(TokenKind::Eof) => &tokens[..tokens.len().saturating_sub(1)],
        _ => tokens,
    };
    let mut seen: HashSet<Next> = HashSet::new();
    apply(rules.iter().flat_map(|r| r.0), t)
        .filter(move |then| seen.insert(*then))
        .map(|then| match then {
            Next::Kw(kw) => vec![kw],
            Next::KwSeq(kws) => kws.to_vec(),
        })
}

fn apply<'a>(
    rules: impl IntoIterator<Item = &'a Rule> + 'a, tokens: &'a [TokenKind],
) -> impl Iterator<Item = Next> + 'a {
    rules
        .into_iter()
        .filter(move |r| r.0.scan(tokens, Dir::Bwd).0)
        .flat_map(|r| r.1.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_if_end() {
        // Forward direction - at end of tokens
        let tokens = vec![TokenKind::Keyword(Keyword::Select)];
        let (matched, offset) = Cond::End.scan_from(&tokens, Dir::Fwd, 1);
        assert!(matched);
        assert_eq!(offset, 1);

        // Forward direction - not at end
        let (matched, offset) = Cond::End.scan_from(&tokens, Dir::Fwd, 0);
        assert!(!matched);
        assert_eq!(offset, 0);

        // Backward direction - empty tokens
        let tokens: Vec<TokenKind> = vec![];
        let (matched, offset) = Cond::End.scan_from(&tokens, Dir::Bwd, 0);
        assert!(matched);
        assert_eq!(offset, 0);

        // Backward direction - non-empty tokens
        let tokens = vec![TokenKind::Keyword(Keyword::Select)];
        let (matched, offset) = Cond::End.scan_from(&tokens, Dir::Bwd, 0);
        assert!(matched);
        assert_eq!(offset, 0);

        // Backward direction
        let tokens = vec![TokenKind::Keyword(Keyword::Select)];
        let (matched, offset) = Cond::End.scan_from(&tokens, Dir::Bwd, 1);
        assert!(!matched);
        assert_eq!(offset, 1);
    }

    #[test]
    fn test_if_kw() {
        let tokens = vec![
            TokenKind::Keyword(Keyword::Select),
            TokenKind::Keyword(Keyword::From),
        ];

        // Match - Forward
        let (matched, offset) = Cond::Kw(Keyword::Select).scan_from(&tokens, Dir::Fwd, 0);
        assert!(matched);
        assert_eq!(offset, 1);

        // No match - Forward
        let (matched, offset) = Cond::Kw(Keyword::Where).scan_from(&tokens, Dir::Fwd, 0);
        assert!(!matched);
        assert_eq!(offset, 0);

        // Match - Backward
        let (matched, offset) = Cond::Kw(Keyword::From).scan_from(&tokens, Dir::Bwd, 2);
        assert!(matched);
        assert_eq!(offset, 1);

        // No match - Backward
        let (matched, offset) = Cond::Kw(Keyword::Select).scan_from(&tokens, Dir::Bwd, 2);
        assert!(!matched);
        assert_eq!(offset, 2);
    }

    #[test]
    fn test_if_op() {
        use crate::lex::Assoc;
        use crate::lex::Fixity;

        let tokens = vec![
            TokenKind::Operator(Operator {
                precedence: 10,
                assoc: Assoc::Left,
                semantic_tag: OpTag::Add,
                fixity: Fixity::Infix,
            }),
            TokenKind::Operator(Operator {
                precedence: 5,
                assoc: Assoc::Left,
                semantic_tag: OpTag::Eq,
                fixity: Fixity::Infix,
            }),
        ];

        // Match - Forward
        let (matched, offset) = Cond::Op(OpTag::Add).scan_from(&tokens, Dir::Fwd, 0);
        assert!(matched);
        assert_eq!(offset, 1);

        // No match - Forward
        let (matched, offset) = Cond::Op(OpTag::Eq).scan_from(&tokens, Dir::Fwd, 0);
        assert!(!matched);
        assert_eq!(offset, 0);

        // Match - Backward
        let (matched, offset) = Cond::Op(OpTag::Eq).scan_from(&tokens, Dir::Bwd, 2);
        assert!(matched);
        assert_eq!(offset, 1);
    }

    #[test]
    fn test_if_kind() {
        let tokens = vec![TokenKind::Identifier, TokenKind::Number, TokenKind::Str];

        // Match - Forward
        let (matched, offset) = Cond::Kind(TokenKind::Identifier).scan_from(&tokens, Dir::Fwd, 0);
        assert!(matched);
        assert_eq!(offset, 1);

        // No match - Forward
        let (matched, offset) = Cond::Kind(TokenKind::Number).scan_from(&tokens, Dir::Fwd, 0);
        assert!(!matched);
        assert_eq!(offset, 0);

        // Match - Backward
        let (matched, offset) = Cond::Kind(TokenKind::Str).scan_from(&tokens, Dir::Bwd, 3);
        assert!(matched);
        assert_eq!(offset, 2);
    }

    #[test]
    fn test_if_any() {
        let tokens = vec![TokenKind::Keyword(Keyword::Select), TokenKind::Identifier];

        // Match first condition
        let any = Cond::Any(&[Cond::Kw(Keyword::Select), Cond::Kw(Keyword::From)]);
        let (matched, offset) = any.scan_from(&tokens, Dir::Fwd, 0);
        assert!(matched);
        assert_eq!(offset, 1);

        // Match second condition
        let any = Cond::Any(&[Cond::Kw(Keyword::From), Cond::Kind(TokenKind::Identifier)]);
        let (matched, offset) = any.scan_from(&tokens, Dir::Fwd, 1);
        assert!(matched);
        assert_eq!(offset, 2);

        // No match
        let any = Cond::Any(&[Cond::Kw(Keyword::Where), Cond::Kw(Keyword::From)]);
        let (matched, offset) = any.scan_from(&tokens, Dir::Fwd, 0);
        assert!(!matched);
        assert_eq!(offset, 0);

        // Backward direction
        let any = Cond::Any(&[Cond::Kw(Keyword::Where), Cond::Kind(TokenKind::Identifier)]);
        let (matched, offset) = any.scan_from(&tokens, Dir::Bwd, 2);
        assert!(matched);
        assert_eq!(offset, 1);
    }

    #[test]
    fn test_if_not() {
        let tokens = vec![TokenKind::Keyword(Keyword::Select), TokenKind::Identifier];

        // Inner matches - negation fails
        let not = Cond::Not(&Cond::Kw(Keyword::Select));
        let (matched, offset) = not.scan_from(&tokens, Dir::Fwd, 0);
        assert!(!matched);
        assert_eq!(offset, 0);

        // Inner doesn't match - negation succeeds
        let not = Cond::Not(&Cond::Kw(Keyword::From));
        let (matched, offset) = not.scan_from(&tokens, Dir::Fwd, 0);
        assert!(matched);
        assert_eq!(offset, 1);

        // Backward direction
        let not = Cond::Not(&Cond::Kw(Keyword::Select));
        let (matched, offset) = not.scan_from(&tokens, Dir::Bwd, 2);
        assert!(matched);
        assert_eq!(offset, 1);
    }

    #[test]
    fn test_if_while() {
        let tokens = vec![
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Keyword(Keyword::Select),
        ];

        // Consume while identifier
        let while_ = Cond::Many(&Cond::Kind(TokenKind::Identifier));
        let (matched, offset) = while_.scan_from(&tokens, Dir::Fwd, 0);
        assert!(matched);
        assert_eq!(offset, 3); // Consumed 3 identifiers

        // Consume while from non-matching position
        let (matched, offset) = while_.scan_from(&tokens, Dir::Fwd, 3);
        assert!(matched);
        assert_eq!(offset, 3); // No consumption, but still "matches"

        let reversed_tokens = tokens.iter().rev().copied().collect::<Vec<_>>();

        // Backward direction
        let while_ = Cond::Many(&Cond::Kind(TokenKind::Identifier));
        let (matched, offset) = while_.scan_from(&reversed_tokens, Dir::Bwd, 4);
        assert!(matched);
        assert_eq!(offset, 1);
    }

    #[test]
    fn test_if_match() {
        let tokens = vec![
            TokenKind::Keyword(Keyword::Select),
            TokenKind::Identifier,
            TokenKind::Keyword(Keyword::From),
        ];

        // Match sequence - Forward
        let match_ = Cond::Seq(&[Cond::Kw(Keyword::Select), Cond::Kind(TokenKind::Identifier)]);
        let (matched, offset) = match_.scan_from(&tokens, Dir::Fwd, 0);
        assert!(matched);
        assert_eq!(offset, 2);

        // Match fails on second element
        let match_ = Cond::Seq(&[Cond::Kw(Keyword::Select), Cond::Kw(Keyword::Where)]);
        let (matched, offset) = match_.scan_from(&tokens, Dir::Fwd, 0);
        assert!(!matched);
        assert_eq!(offset, 1);

        // Match sequence - Backward
        let match_ = Cond::Seq(&[Cond::Kw(Keyword::Select), Cond::Kind(TokenKind::Identifier)]);
        let (matched, offset) = match_.scan_from(&tokens, Dir::Bwd, 2);
        assert!(matched);
        assert_eq!(offset, 0); // Note: moves backward through both tokens
    }

    #[test]
    fn test_if_until() {
        let tokens = vec![
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Keyword(Keyword::From),
            TokenKind::Identifier,
        ];

        // Until keyword - Forward
        let until = Cond::Until(&Cond::Kw(Keyword::From));
        let (matched, offset) = until.scan_from(&tokens, Dir::Fwd, 0);
        assert!(matched);
        assert_eq!(offset, 2); // Stopped at FROM

        // Until not found - Forward
        let until = Cond::Until(&Cond::Kw(Keyword::Where));
        let (matched, offset) = until.scan_from(&tokens, Dir::Fwd, 0);
        assert!(!matched);
        assert_eq!(offset, 4); // Reached end

        let tokens = vec![
            TokenKind::Identifier,
            TokenKind::Keyword(Keyword::From),
            TokenKind::Identifier,
            TokenKind::Identifier,
        ];

        // Until keyword - Backward
        let until = Cond::Until(&Cond::Kw(Keyword::From));
        let (matched, offset) = until.scan_from(&tokens, Dir::Bwd, 4);
        assert!(matched);
        assert_eq!(offset, 2); // Found identifier at position 1
    }
}
