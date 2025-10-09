use crate::token::{OpTag, Token, TokenKind};

#[derive(Debug)]
pub struct TokenTape<'txt, 'tok> {
    pub tokens: &'tok [Token<'txt>],
    pub pos: usize,
}

impl<'txt, 'tok> From<&'tok [Token<'txt>]> for TokenTape<'txt, 'tok> {
    fn from(tokens: &'tok [Token<'txt>]) -> Self {
        Self::new(tokens)
    }
}

impl<'txt, 'tok> From<&'tok Vec<Token<'txt>>> for TokenTape<'txt, 'tok> {
    fn from(tokens: &'tok Vec<Token<'txt>>) -> Self {
        Self::new(tokens)
    }
}

impl<'txt, 'tok> TokenTape<'txt, 'tok> {
    pub fn new(tokens: &'tok [Token<'txt>]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Current token at `pos`.
    pub fn current(&self) -> Option<&Token<'txt>> {
        self.tokens.get(self.pos)
    }

    /// Kind of the current token.
    pub fn current_kind(&self) -> Option<TokenKind> {
        self.current().map(|t| t.kind)
    }

    /// Semantic tag of the current operator.
    pub fn current_operator_tag(&self) -> Option<OpTag> {
        self.current_kind().and_then(|k| match k {
            TokenKind::Operator(op) => Some(op.semantic_tag),
            _ => None,
        })
    }

    /// Previous token
    pub fn prev(&self) -> Option<&Token<'txt>> {
        self.tokens.get(self.pos.saturating_sub(1))
    }

    /// Next token.
    fn peek(&self) -> Option<&Token<'txt>> {
        let idx = self.pos + 1;
        self.tokens.get(idx)
    }

    /// Peek n tokens ahead (n=1 is same as peek())
    pub fn peek_nth(&self, n: usize) -> Option<&Token<'txt>> {
        let idx = self.pos + n;
        self.tokens.get(idx)
    }

    /// Kind of the next token.
    pub fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|t| t.kind)
    }

    /// Is the current token of the given kind?
    pub fn is_at(&self, kind: TokenKind) -> bool {
        self.current_kind() == Some(kind)
    }

    /// Advance and return the current token; `pos` moves forward by one.
    pub fn advance(&mut self) -> Option<&Token<'txt>> {
        if self.pos >= self.tokens.len() {
            return None;
        }
        let i = self.pos;
        self.pos += 1;
        self.tokens.get(i)
    }
}
