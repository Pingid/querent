//! Bidirectional token pattern matching and scanning.
//!
//! This module provides a composable pattern matching system for scanning token
//! sequences in both forward and backward directions. These are used in the
//! dialect module to create keyword completion rules.
//!
//! ### Conditions ([`Cond`])
//!
//! Conditions are pattern matchers that can be composed to express complex
//! token patterns. Basic conditions match individual tokens:
//!
//! ### Direction ([`Dir`])
//!
//! Matching can proceed in two directions:
//!
//! - [`Dir::Fwd`] - Scan forward from start to end of the token sequence
//! - [`Dir::Bwd`] - Scan backward from end to start of the token sequence

use crate::lex::Keyword;
use crate::lex::OpTag;
use crate::lex::Operator;
use crate::lex::TokenKind;

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
    /// Step the offset in the given direction.
    #[inline]
    fn step(&self, offset: usize, len: usize) -> usize {
        match self {
            Dir::Fwd => (offset.saturating_add(1)).min(len),
            Dir::Bwd => offset.saturating_sub(1),
        }
    }
    /// Check if the offset is at the edge of the token sequence.
    #[inline]
    fn at_edge(&self, offset: usize, len: usize) -> bool {
        match self {
            Dir::Fwd => offset >= len,
            Dir::Bwd => offset == 0,
        }
    }
    /// Get the index of the token at the given offset.
    #[inline]
    fn index(&self, offset: usize) -> usize {
        match self {
            Dir::Fwd => offset,
            Dir::Bwd => offset.saturating_sub(1),
        }
    }
    /// Get the start index of the token sequence.
    #[inline]
    fn start(&self, ts: &[TokenKind]) -> usize {
        match self {
            Dir::Fwd => 0,
            Dir::Bwd => ts.len(),
        }
    }
}

impl Cond {
    /// true if the pattern matches starting at the default edge for `dir`.
    pub fn matches(&self, ts: &[TokenKind], dir: Dir) -> bool {
        self.match_from(ts, dir, dir.start(ts)).is_some()
    }

    /// true if the pattern matches and consumes *all* tokens in `dir`.
    pub fn matches_all(&self, ts: &[TokenKind], dir: Dir) -> bool {
        match self.match_from(ts, dir, dir.start(ts)) {
            Some(off) => dir.at_edge(off, ts.len()),
            None => false,
        }
    }

    /// Try to match from an arbitrary offset. On success, returns the new
    /// offset.
    pub fn match_from(&self, ts: &[TokenKind], dir: Dir, offset: usize) -> Option<usize> {
        let mut fuel = ts.len().saturating_mul(64).max(256); // budget
        let (ok, off) = self.match_inner(ts, offset, dir, &mut fuel);
        ok.then_some(off)
    }

    fn match_inner(
        &self, ts: &[TokenKind], offset: usize, direction: Dir, fuel: &mut usize,
    ) -> (bool, usize) {
        if *fuel == 0 {
            return (false, offset);
        }
        *fuel -= 1;

        match self {
            Cond::End => (direction.at_edge(offset, ts.len()), offset),

            Cond::Kw(kw) => match ts.get(direction.index(offset)) {
                Some(t) if t == &TokenKind::Keyword(*kw) => {
                    (true, direction.step(offset, ts.len()))
                }
                _ => (false, offset), // <- don't move on mismatch
            },

            Cond::Op(op) => {
                match ts.get(direction.index(offset)) {
                    Some(TokenKind::Operator(Operator { semantic_tag, .. }))
                        if semantic_tag == op =>
                    {
                        (true, direction.step(offset, ts.len()))
                    }
                    _ => (false, offset), // <- don't move
                }
            }

            Cond::Kind(kind) => match ts.get(direction.index(offset)) {
                Some(t) if t == kind => (true, direction.step(offset, ts.len())),
                _ => (false, offset), // <- don't move
            },

            Cond::Any(ps) => {
                let mut best: Option<usize> = None;
                for p in *ps {
                    let (m, o2) = p.match_inner(ts, offset, direction, fuel);
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
                let (m, _) = if_.match_inner(ts, offset, direction, fuel);
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
                    let (m, next) = if_.match_inner(ts, o, direction, fuel);
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
                            let (m, next) = if_.match_inner(ts, o, direction, fuel);
                            if !m {
                                return (false, o);
                            }
                            o = next;
                        }
                    }
                    Dir::Fwd => {
                        for if_ in ifs.iter() {
                            let (m, next) = if_.match_inner(ts, o, direction, fuel);
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
                    let (m, _) = if_.match_inner(ts, o, direction, fuel);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_if_end() {
        // Forward direction - at end of tokens
        let t = vec![TokenKind::Keyword(Keyword::Select)];
        assert_eq!(Some(1), Cond::End.match_from(&t, Dir::Fwd, 1));

        // Forward direction - not at end
        assert_eq!(None, Cond::End.match_from(&t, Dir::Fwd, 0));

        // Backward direction - empty tokens
        assert_eq!(Some(0), Cond::End.match_from(&vec![], Dir::Bwd, 0));

        // Backward direction - non-empty tokens
        let t = vec![TokenKind::Keyword(Keyword::Select)];
        assert_eq!(Some(0), Cond::End.match_from(&t, Dir::Bwd, 0));

        // Backward direction
        let t = vec![TokenKind::Keyword(Keyword::Select)];
        assert_eq!(None, Cond::End.match_from(&t, Dir::Bwd, 1));
    }

    #[test]
    fn test_if_kw() {
        let t = vec![
            TokenKind::Keyword(Keyword::Select),
            TokenKind::Keyword(Keyword::From),
        ];

        // Match - Forward
        assert_eq!(
            Some(1),
            Cond::Kw(Keyword::Select).match_from(&t, Dir::Fwd, 0)
        );

        // No match - Forward
        assert_eq!(None, Cond::Kw(Keyword::Where).match_from(&t, Dir::Fwd, 0));

        // Match - Backward
        assert_eq!(Some(1), Cond::Kw(Keyword::From).match_from(&t, Dir::Bwd, 2));

        // No match - Backward
        assert_eq!(None, Cond::Kw(Keyword::Select).match_from(&t, Dir::Bwd, 2));
    }

    #[test]
    fn test_if_op() {
        use crate::lex::Assoc;
        use crate::lex::Fixity;

        let t = vec![
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
        assert_eq!(Some(1), Cond::Op(OpTag::Add).match_from(&t, Dir::Fwd, 0));

        // No match - Forward
        assert_eq!(None, Cond::Op(OpTag::Eq).match_from(&t, Dir::Fwd, 0));

        // Match - Backward
        assert_eq!(Some(1), Cond::Op(OpTag::Eq).match_from(&t, Dir::Bwd, 2));
    }

    #[test]
    fn test_if_kind() {
        let t = vec![TokenKind::Identifier, TokenKind::Number, TokenKind::Str];

        // Match - Forward
        assert_eq!(
            Some(1),
            Cond::Kind(TokenKind::Identifier).match_from(&t, Dir::Fwd, 0)
        );

        // No match - Forward
        assert_eq!(
            None,
            Cond::Kind(TokenKind::Number).match_from(&t, Dir::Fwd, 0)
        );

        // Match - Backward
        assert_eq!(
            Some(2),
            Cond::Kind(TokenKind::Str).match_from(&t, Dir::Bwd, 3)
        );
    }

    // #[test]
    // fn test_if_any() {
    //     let tokens = vec![TokenKind::Keyword(Keyword::Select),
    // TokenKind::Identifier];

    //     // Match first condition
    //     let any = Cond::Any(&[Cond::Kw(Keyword::Select),
    // Cond::Kw(Keyword::From)]);     let (matched, offset) =
    // any.match_from(&tokens, Dir::Fwd, 0);     assert!(matched);
    //     assert_eq!(offset, 1);

    //     // Match second condition
    //     let any = Cond::Any(&[Cond::Kw(Keyword::From),
    // Cond::Kind(TokenKind::Identifier)]);     let (matched, offset) =
    // any.match_from(&tokens, Dir::Fwd, 1);     assert!(matched);
    //     assert_eq!(offset, 2);

    //     // No match
    //     let any = Cond::Any(&[Cond::Kw(Keyword::Where),
    // Cond::Kw(Keyword::From)]);     let (matched, offset) =
    // any.match_from(&tokens, Dir::Fwd, 0);     assert!(!matched);
    //     assert_eq!(offset, 0);

    //     // Backward direction
    //     let any = Cond::Any(&[Cond::Kw(Keyword::Where),
    // Cond::Kind(TokenKind::Identifier)]);     let (matched, offset) =
    // any.match_from(&tokens, Dir::Bwd, 2);     assert!(matched);
    //     assert_eq!(offset, 1);
    // }

    // #[test]
    // fn test_if_not() {
    //     let tokens = vec![TokenKind::Keyword(Keyword::Select),
    // TokenKind::Identifier];

    //     // Inner matches - negation fails
    //     let not = Cond::Not(&Cond::Kw(Keyword::Select));
    //     let (matched, offset) = not.match_from(&tokens, Dir::Fwd, 0);
    //     assert!(!matched);
    //     assert_eq!(offset, 0);

    //     // Inner doesn't match - negation succeeds
    //     let not = Cond::Not(&Cond::Kw(Keyword::From));
    //     let (matched, offset) = not.match_from(&tokens, Dir::Fwd, 0);
    //     assert!(matched);
    //     assert_eq!(offset, 1);

    //     // Backward direction
    //     let not = Cond::Not(&Cond::Kw(Keyword::Select));
    //     let (matched, offset) = not.match_from(&tokens, Dir::Bwd, 2);
    //     assert!(matched);
    //     assert_eq!(offset, 1);
    // }

    // #[test]
    // fn test_if_while() {
    //     let tokens = vec![
    //         TokenKind::Identifier,
    //         TokenKind::Identifier,
    //         TokenKind::Identifier,
    //         TokenKind::Keyword(Keyword::Select),
    //     ];

    //     // Consume while identifier
    //     let while_ = Cond::Many(&Cond::Kind(TokenKind::Identifier));
    //     let (matched, offset) = while_.match_from(&tokens, Dir::Fwd, 0);
    //     assert!(matched);
    //     assert_eq!(offset, 3); // Consumed 3 identifiers

    //     // Consume while from non-matching position
    //     let (matched, offset) = while_.match_from(&tokens, Dir::Fwd, 3);
    //     assert!(matched);
    //     assert_eq!(offset, 3); // No consumption, but still "matches"

    //     let reversed_tokens =
    // tokens.iter().rev().copied().collect::<Vec<_>>();

    //     // Backward direction
    //     let while_ = Cond::Many(&Cond::Kind(TokenKind::Identifier));
    //     let (matched, offset) = while_.match_from(&reversed_tokens, Dir::Bwd,
    // 4);     assert!(matched);
    //     assert_eq!(offset, 1);
    // }

    // #[test]
    // fn test_if_match() {
    //     let tokens = vec![
    //         TokenKind::Keyword(Keyword::Select),
    //         TokenKind::Identifier,
    //         TokenKind::Keyword(Keyword::From),
    //     ];

    //     // Match sequence - Forward
    //     let match_ = Cond::Seq(&[Cond::Kw(Keyword::Select),
    // Cond::Kind(TokenKind::Identifier)]);     let (matched, offset) =
    // match_.match_from(&tokens, Dir::Fwd, 0);     assert!(matched);
    //     assert_eq!(offset, 2);

    //     // Match fails on second element
    //     let match_ = Cond::Seq(&[Cond::Kw(Keyword::Select),
    // Cond::Kw(Keyword::Where)]);     let (matched, offset) =
    // match_.match_from(&tokens, Dir::Fwd, 0);     assert!(!matched);
    //     assert_eq!(offset, 1);

    //     // Match sequence - Backward
    //     let match_ = Cond::Seq(&[Cond::Kw(Keyword::Select),
    // Cond::Kind(TokenKind::Identifier)]);     let (matched, offset) =
    // match_.match_from(&tokens, Dir::Bwd, 2);     assert!(matched);
    //     assert_eq!(offset, 0); // Note: moves backward through both
    //     tokens
    // }

    // #[test]
    // fn test_if_until() {
    //     let tokens = vec![
    //         TokenKind::Identifier,
    //         TokenKind::Identifier,
    //         TokenKind::Keyword(Keyword::From),
    //         TokenKind::Identifier,
    //     ];

    //     // Until keyword - Forward
    //     let until = Cond::Until(&Cond::Kw(Keyword::From));
    //     let (matched, offset) = until.match_from(&tokens, Dir::Fwd, 0);
    //     assert!(matched);
    //     assert_eq!(offset, 2); // Stopped at FROM

    //     // Until not found - Forward
    //     let until = Cond::Until(&Cond::Kw(Keyword::Where));
    //     let (matched, offset) = until.match_from(&tokens, Dir::Fwd, 0);
    //     assert!(!matched);
    //     assert_eq!(offset, 4); // Reached end

    //     let tokens = vec![
    //         TokenKind::Identifier,
    //         TokenKind::Keyword(Keyword::From),
    //         TokenKind::Identifier,
    //         TokenKind::Identifier,
    //     ];

    //     // Until keyword - Backward
    //     let until = Cond::Until(&Cond::Kw(Keyword::From));
    //     let (matched, offset) = until.match_from(&tokens, Dir::Bwd, 4);
    //     assert!(matched);
    //     assert_eq!(offset, 2); // Found identifier at position 1
    // }
}
