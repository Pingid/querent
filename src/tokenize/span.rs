use std::ops::Range;

/// Span of a token
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
