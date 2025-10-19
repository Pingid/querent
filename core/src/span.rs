use std::ops::{Deref, Range};

/// A span and an item
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loc<T> {
    pub span: Span,
    pub item: T,
}

impl<T> Loc<T> {
    pub fn new(span: impl Into<Span>, kind: T) -> Self {
        let span = span.into();
        Loc { span, item: kind }
    }
}

impl<T> Deref for Loc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

/// Span of a token
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }
    pub fn contains_inclusive(&self, pos: usize) -> bool {
        pos >= self.start && pos <= self.end
    }
    pub fn is_at_end(&self, pos: usize) -> bool {
        pos == self.end
    }
    pub fn join(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
    pub fn as_str<'a>(&self, input: &'a str) -> &'a str {
        &input[self.start..self.end]
    }
}

impl From<Range<usize>> for Span {
    fn from(r: Range<usize>) -> Self {
        Span {
            start: r.start,
            end: r.end,
        }
    }
}

impl From<usize> for Span {
    fn from(t: usize) -> Self {
        Span { start: t, end: t }
    }
}

impl From<(usize, usize)> for Span {
    fn from(t: (usize, usize)) -> Self {
        Span {
            start: t.0,
            end: t.1,
        }
    }
}
