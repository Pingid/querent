use std::collections::HashSet;

use crate::lex::{Keyword, OpTag, Operator, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleSet(pub &'static [Rule]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule(pub If, pub &'static [Then]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Then {
    Kw(Keyword),
    CombinedKw(&'static [Keyword]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum If {
    /// Match the end of the token stream
    End,
    /// Match a specific keyword
    Kw(Keyword),
    // Match a specific operator
    Op(OpTag),
    /// Match a specific token kind
    Kind(TokenKind),
    /// Match any of a list of conditions
    Any(&'static [If]),
    /// Negate the given if
    Not(&'static If),
    /// Consume while the given if matches
    While(&'static If),
    /// Match all of the given conditions in order
    Match(&'static [If]),
    /// Consume until the given if matches
    Until(&'static If),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Forward,
    Backward,
}

impl Direction {
    /// Move the offset in the specified direction
    fn move_offset(&self, offset: usize, len: usize) -> usize {
        match self {
            Direction::Forward => (offset.saturating_add(1)).min(len),
            Direction::Backward => offset.saturating_sub(1),
        }
    }

    /// Check if we've reached the boundary
    fn at_boundary(&self, offset: usize, len: usize) -> bool {
        match self {
            Direction::Forward => offset >= len,
            Direction::Backward => offset == 0,
        }
    }
}

impl If {
    pub fn match_consume(
        &self,
        tokens: &[TokenKind],
        offset: usize,
        direction: Direction,
    ) -> (bool, usize) {
        let mut fuel = tokens.len().saturating_mul(64).max(256); // budget
        self.match_consume_inner(tokens, offset, direction, &mut fuel)
    }

    fn match_consume_inner(
        &self,
        tokens: &[TokenKind],
        offset: usize,
        direction: Direction,
        fuel: &mut usize,
    ) -> (bool, usize) {
        if *fuel == 0 {
            return (false, offset);
        }
        *fuel -= 1;

        match self {
            If::End => {
                let at_end = match direction {
                    Direction::Forward => offset >= tokens.len(),
                    Direction::Backward => tokens.is_empty(),
                };
                (at_end, offset)
            }

            If::Kw(kw) => match tokens.get(offset) {
                Some(t) if t == &TokenKind::Keyword(*kw) => {
                    (true, direction.move_offset(offset, tokens.len()))
                }
                _ => (false, offset), // <- don't move on mismatch
            },

            If::Op(op) => match tokens.get(offset) {
                Some(TokenKind::Operator(Operator { semantic_tag, .. })) if semantic_tag == op => {
                    (true, direction.move_offset(offset, tokens.len()))
                }
                _ => (false, offset), // <- don't move
            },

            If::Kind(kind) => match tokens.get(offset) {
                Some(t) if t == kind => (true, direction.move_offset(offset, tokens.len())),
                _ => (false, offset), // <- don't move
            },

            If::Any(ps) => {
                let mut best: Option<usize> = None;
                for p in *ps {
                    let (m, o2) = p.match_consume_inner(tokens, offset, direction, fuel);
                    if !m {
                        continue;
                    }
                    best = Some(match (best, direction) {
                        (None, _) => o2,
                        (Some(o_best), Direction::Forward) => o2.max(o_best),
                        (Some(o_best), Direction::Backward) => o2.min(o_best),
                    });
                }
                match best {
                    Some(o) => (true, o),
                    None => (false, offset),
                }
            }

            If::Not(if_) => {
                let (m, _) = if_.match_consume_inner(tokens, offset, direction, fuel);
                match m {
                    // inner matched -> negation fails, no consumption
                    true => (false, offset),
                    // inner didn't match -> negation succeeds, consume current token
                    false => (true, direction.move_offset(offset, tokens.len())),
                }
            }

            If::While(if_) => {
                let mut o = offset;
                loop {
                    let (m, next) = if_.match_consume_inner(tokens, o, direction, fuel);
                    if !m || next == o {
                        break;
                    }
                    o = next;
                }
                (true, o)
            }

            If::Match(ifs) => {
                let mut o = offset;
                match direction {
                    Direction::Backward => {
                        for if_ in ifs.iter().rev() {
                            let (m, next) = if_.match_consume_inner(tokens, o, direction, fuel);
                            if !m {
                                return (false, o);
                            }
                            o = next;
                        }
                    }
                    Direction::Forward => {
                        for if_ in ifs.iter() {
                            let (m, next) = if_.match_consume_inner(tokens, o, direction, fuel);
                            if !m {
                                return (false, o);
                            }
                            o = next;
                        }
                    }
                }
                (true, o)
            }

            If::Until(if_) => {
                let mut o = offset;
                loop {
                    let (m, _) = if_.match_consume_inner(tokens, o, direction, fuel);
                    if m {
                        return (true, o);
                    }
                    if direction.at_boundary(o, tokens.len()) {
                        return (false, o);
                    }
                    let next = direction.move_offset(o, tokens.len());
                    if next == o {
                        return (false, o);
                    }
                    o = next;
                }
            }
        }
    }
}

pub fn resolve_follow_rules<'a>(
    rules: &'a [RuleSet],
    tokens: &'a [TokenKind],
) -> impl Iterator<Item = String> + 'a {
    let t = match tokens.last() {
        Some(TokenKind::Eof) => &tokens[..tokens.len().saturating_sub(1)],
        _ => tokens,
    };
    let mut seen: HashSet<Then> = HashSet::new();
    get_matches(rules.iter().flat_map(|r| r.0), t, t.len().saturating_sub(1))
        .filter(move |then| seen.insert(*then))
        .map(|then| format!("{then}").to_uppercase())
}

fn get_matches<'a>(
    rules: impl IntoIterator<Item = &'a Rule> + 'a,
    tokens: &'a [TokenKind],
    offset: usize,
) -> impl Iterator<Item = Then> + 'a {
    rules
        .into_iter()
        .filter(move |r| r.0.match_consume(tokens, offset, Direction::Backward).0)
        .flat_map(|r| r.1.iter().copied())
}

impl std::fmt::Display for Then {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Then::Kw(kw) => write!(f, "{:?}", kw).map(|_| ()), // caller can upper if needed
            Then::CombinedKw(kws) => {
                let mut first = true;
                for kw in *kws {
                    if !first {
                        write!(f, " ")?;
                    }
                    write!(f, "{:?}", kw)?;
                    first = false;
                }
                Ok(())
            }
        }
    }
}
