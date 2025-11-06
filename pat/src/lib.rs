//! Bidirectional pattern matching with tiny, composable combinators.
//!
//! Works over any token slice and can run **forward** or **backward** via a
//! const generic. Core pieces:
//!
//! - [`Pat`]: enum of pattern combinators
//! - [`Cursor`]: bidirectional cursor over a slice (with fuel to prevent loops)
//! - [`Pattern`]: trait for things that can consume from a [`Cursor`]
//!
//! # Combinators
//! - `Atom`: match with a predicate/pattern `P`
//! - `Eof`: match end of input
//! - `Not`: succeed if inner pattern fails; consumes one token on success
//! - `Or`: try patterns left‑to‑right; return the first success
//! - `Longest`: try all and take the furthest‑consuming success
//! - `Many`: repeat until it stops matching; always succeeds
//! - `Opt`: match zero or one occurrence; always succeeds
//! - `Seq`: all patterns must match in order
//! - `Until`: consume until inner pattern would match (that match is not
//!   consumed)
//! - `Separated`: match zero or more items separated by a separator
//! - `Scoped`: match content within opening and closing patterns, handling
//!   nesting
//!
//! # Example
//! ```rust
//! use pat::{Pat, Predicate, match_all};
//!
//! struct Char(char);
//!
//! impl Predicate for Char {
//! type Token = char;
//! fn test(&self, t: &Self::Token) -> bool { self.0 == *t }
//! }
//!
//! static ABC: [Pat<Char>; 3] = [
//! Pat::atom(Char('a')),
//! Pat::atom(Char('b')),
//! Pat::atom(Char('c')),
//! ];
//!
//! let input = ['a', 'b', 'c', 'd'];
//! let res = match_all::<false, _>(&Pat::seq(&ABC), &input);
//! assert!(res.is_ok());
//! ```

/// Composable pattern combinators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pat<'a, P: 'a> {
    /// Match using and consumes the token.
    Atom(P),
    /// Match and dont consume the token.
    Peek(&'a Pat<'a, P>),
    /// Match end of input.
    Eof,
    /// Match 1 token.
    Any,
    /// Match without consuming tokens
    Empty,
    /// Succeeds when `cond` fails; on success, consumes one token.
    Not(&'a Pat<'a, P>),
    /// Try each pattern in order; return the first success.
    Or(&'a [Pat<'a, P>]),
    /// Try all and pick the furthest‑consuming success.
    Longest(&'a [Pat<'a, P>]),
    /// Repeat `cond` while it matches; always succeeds.
    Many(&'a Pat<'a, P>),
    /// Match zero or one occurrence; always succeeds.
    Opt(&'a Pat<'a, P>),
    /// All patterns must match in order.
    Seq(&'a [Pat<'a, P>]),
    /// Consume until `cond` would match; does not consume that match.
    Until(&'a Pat<'a, P>),
    // Consume until consumes match
    UntilIncl(&'a Pat<'a, P>),
    /// Match a list of patterns separated by a separator.
    Separated(&'a Pat<'a, P>, &'a Pat<'a, P>), // separator, pattern
    /// Match a pattern inside a scoped block.
    Scoped(&'a Pat<'a, P>, &'a Pat<'a, P>), // open, close
}

impl<'a, P: 'a> Pat<'a, P> {
    pub const fn atom(p: P) -> Self {
        Self::Atom(p)
    }

    pub const fn peek(p: &'static Pat<P>) -> Self {
        Self::Peek(p)
    }

    pub const fn eof() -> Self {
        Self::Eof
    }

    pub const fn empty() -> Self {
        Self::Empty
    }

    pub const fn not(cond: &'static Pat<P>) -> Self {
        Self::Not(cond)
    }

    pub const fn or(conds: &'static [Pat<P>]) -> Self {
        Self::Or(conds)
    }

    pub const fn longest(conds: &'static [Pat<P>]) -> Self {
        Self::Longest(conds)
    }

    pub const fn many(cond: &'static Pat<P>) -> Self {
        Self::Many(cond)
    }

    pub const fn opt(cond: &'static Pat<P>) -> Self {
        Self::Opt(cond)
    }

    pub const fn seq(conds: &'static [Pat<P>]) -> Self {
        Self::Seq(conds)
    }

    pub const fn until(cond: &'static Pat<P>) -> Self {
        Self::Until(cond)
    }

    pub const fn until_incl(cond: &'static Pat<P>) -> Self {
        Self::UntilIncl(cond)
    }

    pub const fn separated(sep: &'static Pat<P>, item: &'static Pat<P>) -> Self {
        Self::Separated(sep, item)
    }

    pub const fn scoped(open: &'static Pat<P>, close: &'static Pat<P>) -> Self {
        Self::Scoped(open, close)
    }
}

impl<'a, T, P: 'a, const BACKWARD: bool> Pattern<BACKWARD> for Pat<'a, P>
where
    P: Pattern<BACKWARD, Token = T>,
{
    type Token = T;

    fn match_one(&self, cursor: &mut Cursor<'_, T, BACKWARD>) -> Match {
        let start = cursor.pos;

        let matched = |pos: usize| match BACKWARD {
            true => Match::Match((pos, start)),
            false => Match::Match((start, pos)),
        };

        match self {
            Pat::Eof => match cursor.is_exhausted() {
                true => matched(cursor.pos),
                false => Match::NoMatch,
            },
            Pat::Atom(p) => p.match_one(cursor),
            Pat::Peek(p) => {
                let pos = cursor.pos;
                match p.match_one(cursor) {
                    Match::Match(_) => {
                        cursor.pos = pos;
                        Match::Match((pos, pos))
                    }
                    Match::NoMatch => Match::NoMatch,
                    Match::Eof => Match::Eof,
                }
            }
            Pat::Any => {
                cursor.advance();
                matched(cursor.pos)
            }
            Pat::Empty => Match::Match((start, cursor.pos)),
            Pat::Not(cond) => {
                let pos = cursor.pos;
                match cond.match_one(cursor) {
                    Match::Match(_) => {
                        cursor.pos = pos;
                        Match::NoMatch
                    }
                    Match::NoMatch => match cursor.peek().is_some() && cursor.advance() {
                        true => matched(cursor.pos),
                        false => Match::NoMatch,
                    },
                    Match::Eof => Match::Eof,
                }
            }

            Pat::Or(conds) => {
                for i in 0..conds.len() {
                    let i = if BACKWARD { conds.len() - 1 - i } else { i };
                    match conds[i].match_one(cursor) {
                        ok @ Match::Match(_) => return ok,
                        _ => cursor.pos = start,
                    }
                }
                Match::NoMatch
            }

            Pat::Longest(conds) => {
                let mut best: Option<(usize, usize)> = None;
                for c in *conds {
                    let res = c.match_one(cursor);
                    match res {
                        Match::Match((s, e)) => {
                            best = Some(match (best, BACKWARD) {
                                (None, _) => (s, e),
                                (Some((_, be)), false) if e > be => (s, e),
                                (Some((bs, be)), true) if s.min(e) < bs.min(be) => (s, e),
                                (Some(prev), _) => prev,
                            });
                            cursor.pos = start;
                        }
                        _ => {
                            cursor.pos = start;
                        }
                    }
                }
                match best {
                    Some((s, e)) => {
                        cursor.pos = if BACKWARD { s.min(e) } else { s.max(e) };
                        Match::Match((s, e))
                    }
                    None => Match::NoMatch,
                }
            }

            Pat::Many(cond) => {
                let mut prev = cursor.pos;
                while let Match::Match(_) = cond.match_one(cursor) {
                    if cursor.pos == prev || cursor.is_exhausted() {
                        break;
                    }
                    prev = cursor.pos;
                }
                matched(cursor.pos)
            }

            Pat::Opt(cond) => {
                // Try to match once, but always succeed
                match cond.match_one(cursor) {
                    Match::Match(_) => matched(cursor.pos),
                    _ => matched(start), // No match is ok, return original position
                }
            }

            Pat::Seq(conds) => {
                for i in 0..conds.len() {
                    let i = if BACKWARD { conds.len() - 1 - i } else { i };
                    match conds[i].match_one(cursor) {
                        Match::Match(_) => continue,
                        m => return m,
                    }
                }
                matched(cursor.pos)
            }

            Pat::Until(cond) => {
                let mut prev = cursor.pos;
                loop {
                    match cond.match_one(cursor) {
                        Match::Match(_) => {
                            cursor.pos = prev;
                            return match BACKWARD {
                                true => Match::Match((prev, start)),
                                false => Match::Match((start, prev)),
                            };
                        }
                        Match::Eof => return Match::NoMatch,
                        Match::NoMatch => {
                            if cursor.peek().is_some() && cursor.advance() {
                                prev = cursor.pos;
                            } else {
                                return Match::Eof;
                            }
                        }
                    }
                }
            }
            Pat::UntilIncl(cond) => loop {
                match cond.match_one(cursor) {
                    Match::Match(_) => return matched(cursor.pos),
                    Match::Eof => return Match::NoMatch,
                    Match::NoMatch => match cursor.peek().is_some() && cursor.advance() {
                        true => continue,
                        false => return Match::Eof,
                    },
                }
            },

            Pat::Separated(sep, item) => {
                // Try to match first item (if it doesn't match, return success with 0 items)
                match item.match_one(cursor) {
                    Match::Match(_) => {}
                    _ => return matched(start), // Zero items is ok
                }

                // After first item, repeatedly match: separator followed by item
                loop {
                    let pos = cursor.pos;
                    match sep.match_one(cursor) {
                        Match::Match(_) => match item.match_one(cursor) {
                            Match::Match(_) => continue,
                            _ => {
                                // Separator matched but item didn't, restore position
                                cursor.pos = pos;
                                return matched(cursor.pos);
                            }
                        },
                        _ => return matched(cursor.pos),
                    }
                }
            }
            Pat::Scoped(left, right) => {
                let open = match BACKWARD {
                    true => right,
                    false => left,
                };
                let close = match BACKWARD {
                    true => left,
                    false => right,
                };
                // Match opening pattern
                match open.match_one(cursor) {
                    Match::Match(_) => {}
                    m => return m,
                }

                // Track nesting depth
                let mut depth = 1;

                // Consume tokens until we find the matching close
                while depth > 0 {
                    if cursor.is_exhausted() {
                        return Match::Eof;
                    }

                    let pos = cursor.pos;

                    // Try to match opening pattern (increases depth)
                    match open.match_one(cursor) {
                        Match::Match(_) => {
                            depth += 1;
                            continue;
                        }
                        _ => cursor.pos = pos,
                    }

                    // Try to match closing pattern (decreases depth)
                    match close.match_one(cursor) {
                        Match::Match(_) => {
                            depth -= 1;
                            if depth == 0 {
                                return matched(cursor.pos);
                            }
                            continue;
                        }
                        _ => cursor.pos = pos,
                    }

                    // Neither matched, advance one token
                    if !cursor.advance() {
                        return Match::Eof;
                    }
                }

                matched(cursor.pos)
            }
        }
    }
}

/// A thing that can match tokens from a [`Cursor`].
pub trait Pattern<const BACKWARD: bool> {
    /// The token type.
    type Token;
    /// Try to match tokens from the given [`Cursor`]; return the span or a
    /// failure kind.
    fn match_one(&self, cursor: &mut Cursor<'_, Self::Token, BACKWARD>) -> Match;
}

/// Result of a single match attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Match {
    /// Matched a span `(start, end)`.
    Match((usize, usize)),
    /// Pattern did not match here.
    NoMatch,
    /// Hit end of input while attempting to match.
    Eof,
}

impl Match {
    pub fn is_ok(&self) -> bool {
        matches!(self, Match::Match(_))
    }

    pub fn is_no(&self) -> bool {
        matches!(self, Match::NoMatch)
    }

    pub fn is_eof(&self) -> bool {
        matches!(self, Match::Eof)
    }
}

pub trait Predicate {
    type Token;
    fn test(&self, t: &Self::Token) -> bool;
}

impl<T, P, const BACKWARD: bool> Pattern<BACKWARD> for P
where
    P: Predicate<Token = T>,
{
    type Token = T;

    #[inline(always)]
    fn match_one(&self, cursor: &mut Cursor<'_, T, BACKWARD>) -> Match {
        let start = cursor.pos;
        match cursor.peek() {
            Some(item) if self.test(item) => {
                if cursor.advance() {
                    match BACKWARD {
                        true => Match::Match((cursor.pos, start)),
                        false => Match::Match((start, cursor.pos)),
                    }
                } else {
                    Match::Eof
                }
            }
            Some(_) => Match::NoMatch,
            None => Match::Eof,
        }
    }
}

/// Match from a given `offset`.
pub fn match_from<const BACKWARD: bool, P>(p: &Pat<P>, items: &[P::Token], offset: usize) -> Match
where
    P: Pattern<BACKWARD>,
{
    let mut cursor = Cursor::<_, BACKWARD>::new(items);
    cursor.pos = offset;
    p.match_one(&mut cursor)
}

pub fn match_all<const BACKWARD: bool, P>(p: &Pat<P>, items: &[P::Token]) -> Match
where
    P: Pattern<BACKWARD>,
{
    p.match_one(&mut Cursor::<_, BACKWARD>::new(items))
}

/// Cursor over a slice; direction is set by the `BACKWARD` const generic.
pub struct Cursor<'a, T, const BACKWARD: bool = false> {
    /// Entire input.
    pub items: &'a [T],
    /// Current position (index where next read happens).
    pub pos: usize,
    /// Step budget to avoid pathological loops.
    pub fuel: usize,
}

impl<'a, T, const BACKWARD: bool> Cursor<'a, T, BACKWARD> {
    /// Create a cursor at start (forward) or end (backward).
    pub fn new(items: &'a [T]) -> Self {
        let pos = if BACKWARD { items.len() } else { 0 };
        Self {
            items,
            pos,
            fuel: items.len().saturating_mul(64).max(256),
        }
    }

    /// Peek the current item without moving.
    #[inline(always)]
    pub fn peek(&self) -> Option<&T> {
        if BACKWARD {
            // For backward mode, peek at pos-1, but when pos=0, peek at 0 to match legacy
            // behavior
            let idx = self.pos.saturating_sub(1);
            self.items.get(idx)
        } else {
            self.items.get(self.pos)
        }
    }

    /// Advance by one. Returns `false` only when out of fuel (no move).
    #[inline(always)]
    pub fn advance(&mut self) -> bool {
        if self.fuel == 0 {
            return false;
        }
        self.fuel -= 1;
        if BACKWARD {
            self.pos = self.pos.saturating_sub(1);
        } else {
            self.pos = (self.pos + 1).min(self.items.len());
        }
        true
    }

    /// Returns `true` if there are no items left in this direction.
    #[inline(always)]
    pub fn is_exhausted(&self) -> bool {
        if BACKWARD {
            self.pos == 0
        } else {
            self.pos >= self.items.len()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tok(char);

    impl Predicate for Tok {
        type Token = char;
        fn test(&self, token: &Self::Token) -> bool {
            self.0 == *token
        }
    }

    #[test]
    fn test_if_end() {
        // Forward direction - at end of tokens
        assert_eq!(
            Match::Match((1, 1)),
            match_from::<false, Tok>(&Pat::Eof, &['a'], 1)
        );

        // Forward direction - not at end
        assert_eq!(
            Match::NoMatch,
            match_from::<false, Tok>(&Pat::Eof, &['a'], 0)
        );

        // Backward direction - empty tokens
        assert_eq!(Match::Match((0, 0)), match_all::<true, Tok>(&Pat::Eof, &[]));

        // Backward direction - non-empty tokens
        assert_eq!(
            Match::Match((0, 0)),
            match_from::<true, Tok>(&Pat::Eof, &['a'], 0)
        );

        // Backward direction
        assert_eq!(Match::NoMatch, match_all::<true, Tok>(&Pat::Eof, &['a']));
    }

    #[test]
    fn test_pat() {
        // Forward direction - matching pattern
        assert_eq!(
            Match::Match((0, 1)),
            match_all::<false, Tok>(&Pat::Atom(Tok('a')), &['a', 'b', 'c'])
        );

        // Forward direction - non-matching pattern
        assert_eq!(
            Match::NoMatch,
            match_all::<false, Tok>(&Pat::Atom(Tok('x')), &['a', 'b', 'c'])
        );

        // Forward direction - exhausted
        assert_eq!(
            Match::Eof,
            match_from::<false, Tok>(&Pat::Atom(Tok('a')), &['a', 'b'], 2)
        );

        // Backward direction - matching pattern
        assert_eq!(
            Match::Match((2, 3)),
            match_from::<true, Tok>(&Pat::Atom(Tok('c')), &['a', 'b', 'c'], 3)
        );

        // Backward direction - non-matching pattern
        assert_eq!(
            Match::NoMatch,
            match_from::<true, Tok>(&Pat::Atom(Tok('x')), &['a', 'b', 'c'], 3)
        );
    }

    #[test]
    fn test_not() {
        static NOT_A: Pat<Tok> = Pat::Not(&Pat::Atom(Tok('a')));
        static NOT_NOT_A: Pat<Tok> = Pat::Not(&NOT_A);

        // Forward direction - pattern doesn't match (Not succeeds)
        assert_eq!(
            Match::Match((0, 1)),
            match_all::<false, Tok>(&NOT_A, &['b', 'c', 'd'])
        );

        // Forward direction - pattern matches (Not fails)
        assert_eq!(
            Match::NoMatch,
            match_all::<false, Tok>(&NOT_A, &['a', 'b', 'c'])
        );

        // Forward direction - exhausted
        assert_eq!(Match::Eof, match_from::<false, Tok>(&NOT_A, &['a'], 1));

        // Backward direction - pattern doesn't match (Not succeeds)
        assert_eq!(
            Match::Match((2, 3)),
            match_from::<true, Tok>(&NOT_A, &['b', 'c', 'd'], 3)
        );

        // Nested Not
        assert_eq!(
            Match::Match((0, 1)),
            match_all::<false, Tok>(&NOT_NOT_A, &['a', 'b', 'c'])
        );
    }

    #[test]
    fn test_many() {
        static PAT_A: Pat<Tok> = Pat::Atom(Tok('a'));
        static PAT_B: Pat<Tok> = Pat::Atom(Tok('b'));

        // Forward direction - multiple matches
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<false, Tok>(&Pat::Many(&PAT_A), &['a', 'a', 'a', 'b'])
        );

        // Forward direction - zero matches (always succeeds)
        assert_eq!(
            Match::Match((0, 0)),
            match_all::<false, Tok>(&Pat::Many(&PAT_A), &['b', 'c', 'd'])
        );

        // Forward direction - consumes all matching tokens
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<false, Tok>(&Pat::Many(&PAT_A), &['a', 'a', 'a', 'b', 'c'])
        );

        // Backward direction - multiple matches
        assert_eq!(
            Match::Match((1, 4)),
            match_all::<true, Tok>(&Pat::Many(&PAT_A), &['b', 'a', 'a', 'a'])
        );

        // Forward direction - stops when pattern no longer matches
        assert_eq!(
            Match::Match((1, 4)),
            match_from::<false, Tok>(&Pat::Many(&PAT_B), &['x', 'b', 'b', 'b', 'c'], 1)
        );
    }

    #[test]
    fn test_opt() {
        static PAT_A: Pat<Tok> = Pat::Atom(Tok('a'));
        static PAT_B: Pat<Tok> = Pat::Atom(Tok('b'));
        static OPT_A: Pat<Tok> = Pat::Opt(&PAT_A);

        // Forward direction - matches one occurrence
        assert_eq!(
            Match::Match((0, 1)),
            match_all::<false, Tok>(&OPT_A, &['a', 'b', 'c'])
        );

        // Forward direction - zero matches (always succeeds)
        assert_eq!(
            Match::Match((0, 0)),
            match_all::<false, Tok>(&OPT_A, &['b', 'c', 'd'])
        );

        // Forward direction - only matches once, not multiple
        assert_eq!(
            Match::Match((0, 1)),
            match_all::<false, Tok>(&OPT_A, &['a', 'a', 'a'])
        );

        // Backward direction - matches one occurrence
        assert_eq!(
            Match::Match((2, 3)),
            match_from::<true, Tok>(&OPT_A, &['b', 'c', 'a'], 3)
        );

        // Backward direction - zero matches (always succeeds, cursor stays at start)
        assert_eq!(
            Match::Match((3, 3)),
            match_all::<true, Tok>(&OPT_A, &['b', 'c', 'd'])
        );

        // Use in sequence - optional element
        static SEQ_WITH_OPT: [Pat<Tok>; 3] =
            [Pat::Atom(Tok('a')), Pat::Opt(&PAT_B), Pat::Atom(Tok('c'))];

        // Sequence with optional present
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<false, Tok>(&Pat::Seq(&SEQ_WITH_OPT), &['a', 'b', 'c'])
        );

        // Sequence with optional absent
        assert_eq!(
            Match::Match((0, 2)),
            match_all::<false, Tok>(&Pat::Seq(&SEQ_WITH_OPT), &['a', 'c'])
        );

        // Empty input
        assert_eq!(Match::Match((0, 0)), match_all::<false, Tok>(&OPT_A, &[]));
    }

    #[test]
    fn test_seq() {
        static SEQ_ABC: [Pat<Tok>; 3] = [
            Pat::Atom(Tok('a')),
            Pat::Atom(Tok('b')),
            Pat::Atom(Tok('c')),
        ];
        static EMPTY_SEQ: [Pat<Tok>; 0] = [];

        // Forward direction - all patterns match in sequence
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<false, Tok>(&Pat::Seq(&SEQ_ABC), &['a', 'b', 'c'])
        );

        // Forward direction - first pattern doesn't match
        assert_eq!(
            Match::NoMatch,
            match_all::<false, Tok>(&Pat::Seq(&SEQ_ABC), &['b', 'b', 'c'])
        );

        // Forward direction - middle pattern doesn't match
        assert_eq!(
            Match::NoMatch,
            match_all::<false, Tok>(&Pat::Seq(&SEQ_ABC), &['a', 'a', 'c'])
        );

        // Forward direction - last pattern doesn't match
        assert_eq!(
            Match::NoMatch,
            match_all::<false, Tok>(&Pat::Seq(&SEQ_ABC), &['a', 'b', 'b'])
        );

        // Backward direction - all patterns match in reverse
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<true, Tok>(&Pat::Seq(&SEQ_ABC), &['a', 'b', 'c'])
        );

        // Backward direction - first pattern doesn't match
        assert_eq!(
            Match::NoMatch,
            match_all::<true, Tok>(&Pat::Seq(&SEQ_ABC), &['a', 'b', 'b'])
        );

        // Empty sequence
        assert_eq!(
            Match::Match((0, 0)),
            match_all::<false, Tok>(&Pat::Seq(&EMPTY_SEQ), &['a', 'b', 'c'])
        );
    }

    #[test]
    fn test_until() {
        static PAT_A: Pat<Tok> = Pat::Atom(Tok('a'));
        static PAT_Z: Pat<Tok> = Pat::Atom(Tok('z'));
        static END: Pat<Tok> = Pat::Eof;

        // Forward direction - stops at matching pattern
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<false, Tok>(&Pat::Until(&PAT_A), &['x', 'y', 'z', 'a', 'b'])
        );

        // Forward direction - pattern never matches
        assert_eq!(
            Match::NoMatch,
            match_all::<false, Tok>(&Pat::Until(&PAT_Z), &['a', 'b', 'c'])
        );

        // Forward direction - pattern matches immediately
        assert_eq!(
            Match::Match((0, 0)),
            match_all::<false, Tok>(&Pat::Until(&PAT_A), &['a', 'b', 'c'])
        );

        // Backward direction - stops at matching pattern
        assert_eq!(
            Match::Match((2, 5)),
            match_all::<true, Tok>(&Pat::Until(&PAT_A), &['x', 'a', 'b', 'c', 'd'])
        );

        // Forward direction - with complex pattern (End)
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<false, Tok>(&Pat::Until(&END), &['a', 'b', 'c'])
        );
    }

    #[test]
    fn test_separated_list() {
        static PAT_A: Pat<Tok> = Pat::Atom(Tok('a'));
        static PAT_COMMA: Pat<Tok> = Pat::Atom(Tok(','));
        static PAT_B: Pat<Tok> = Pat::Atom(Tok('b'));
        static LIST_A_COMMA: Pat<Tok> = Pat::Separated(&PAT_COMMA, &PAT_A);
        static LIST_B_COMMA: Pat<Tok> = Pat::Separated(&PAT_COMMA, &PAT_B);

        // Forward direction - single item
        assert_eq!(
            Match::Match((0, 1)),
            match_all::<false, Tok>(&LIST_A_COMMA, &['a'])
        );

        // Forward direction - two items separated by comma
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<false, Tok>(&LIST_A_COMMA, &['a', ',', 'a'])
        );

        // Forward direction - three items separated by commas
        assert_eq!(
            Match::Match((0, 5)),
            match_all::<false, Tok>(&LIST_A_COMMA, &['a', ',', 'a', ',', 'a'])
        );

        // Forward direction - stops at first non-matching item
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<false, Tok>(&LIST_A_COMMA, &['a', ',', 'a', 'b'])
        );

        // Forward direction - stops when separator is present but item doesn't match
        assert_eq!(
            Match::Match((0, 1)),
            match_all::<false, Tok>(&LIST_A_COMMA, &['a', ',', 'b'])
        );

        // Forward direction - zero items when first item doesn't match
        assert_eq!(
            Match::Match((0, 0)),
            match_all::<false, Tok>(&LIST_A_COMMA, &['b', ',', 'a'])
        );

        // Forward direction - trailing separator doesn't consume extra tokens
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<false, Tok>(&LIST_A_COMMA, &['a', ',', 'a', ','])
        );

        // Backward direction - single item
        assert_eq!(
            Match::Match((0, 1)),
            match_all::<true, Tok>(&LIST_B_COMMA, &['b'])
        );

        // Backward direction - two items separated by comma
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<true, Tok>(&LIST_B_COMMA, &['b', ',', 'b'])
        );

        // Backward direction - three items
        assert_eq!(
            Match::Match((0, 5)),
            match_all::<true, Tok>(&LIST_B_COMMA, &['b', ',', 'b', ',', 'b'])
        );

        // Backward direction - stops at first non-matching item (going backwards from
        // end) Input: ['a', 'b', ',', 'b']
        // Backward: matches 'b', then ',', then 'b' (stops at 'a' which doesn't match
        // separator)
        assert_eq!(
            Match::Match((1, 4)),
            match_all::<true, Tok>(&LIST_B_COMMA, &['a', 'b', ',', 'b'])
        );

        // Empty input - now succeeds with zero items
        assert_eq!(
            Match::Match((0, 0)),
            match_all::<false, Tok>(&LIST_A_COMMA, &[])
        );
    }

    #[test]
    fn test_separated_list_complex() {
        // Test with more complex patterns - using Or to match either 'a' or 'b'
        static PAT_COMMA: Pat<Tok> = Pat::Atom(Tok(','));
        static OR_AB: [Pat<Tok>; 2] = [Pat::Atom(Tok('a')), Pat::Atom(Tok('b'))];
        static PAT_OR_AB: Pat<Tok> = Pat::Or(&OR_AB);
        static LIST_OR_COMMA: Pat<Tok> = Pat::Separated(&PAT_COMMA, &PAT_OR_AB);

        // Forward direction - mixed items
        assert_eq!(
            Match::Match((0, 5)),
            match_all::<false, Tok>(&LIST_OR_COMMA, &['a', ',', 'b', ',', 'a'])
        );

        // Forward direction - all 'a' items
        assert_eq!(
            Match::Match((0, 5)),
            match_all::<false, Tok>(&LIST_OR_COMMA, &['a', ',', 'a', ',', 'a'])
        );

        // Forward direction - all 'b' items
        assert_eq!(
            Match::Match((0, 5)),
            match_all::<false, Tok>(&LIST_OR_COMMA, &['b', ',', 'b', ',', 'b'])
        );

        // Forward direction - stops at non-matching separator
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<false, Tok>(&LIST_OR_COMMA, &['a', ',', 'b', ';', 'a'])
        );
    }

    #[test]
    fn test_separated_list_edge_cases() {
        static PAT_A: Pat<Tok> = Pat::Atom(Tok('a'));
        static PAT_COMMA: Pat<Tok> = Pat::Atom(Tok(','));
        static LIST: Pat<Tok> = Pat::Separated(&PAT_COMMA, &PAT_A);

        // Multiple consecutive separators without items
        assert_eq!(
            Match::Match((0, 1)),
            match_all::<false, Tok>(&LIST, &['a', ',', ',', 'a'])
        );

        // Separator without following item at end of input
        assert_eq!(
            Match::Match((0, 1)),
            match_all::<false, Tok>(&LIST, &['a', ','])
        );

        // Zero items at start of non-empty input
        assert_eq!(
            Match::Match((0, 0)),
            match_all::<false, Tok>(&LIST, &['b', 'c', 'd'])
        );
    }

    #[test]
    fn test_scoped() {
        static PAT_SCOPED: Pat<Tok> = Pat::Scoped(&Pat::Atom(Tok('(')), &Pat::Atom(Tok(')')));
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<false, Tok>(&PAT_SCOPED, &['(', 'a', ')'])
        );
        assert_eq!(
            Match::Match((0, 3)),
            match_all::<true, Tok>(&PAT_SCOPED, &['(', 'a', ')'])
        );
        assert_eq!(
            Match::Match((0, 6)),
            match_all::<false, Tok>(&PAT_SCOPED, &['(', 'b', '(', 'a', ')', ')'])
        );
        assert_eq!(
            Match::Match((0, 6)),
            match_all::<true, Tok>(&PAT_SCOPED, &['(', 'b', '(', 'a', ')', ')'])
        );
    }
}
