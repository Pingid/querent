use crate::dialect::{CommentStyle, DialectSpec};
use crate::span::Span;
use crate::token::QuoteStyle;
use crate::token::Token;
use crate::token::TokenKind;

pub struct Lexer<'txt, 'spec> {
    spec: &'spec DialectSpec,
    input: &'txt str,
    cursor: usize,
    done: bool,

    // longest keyword and operator length for greedy match (computed once)
    max_op_len: usize,
}

impl<'txt, 'spec> Lexer<'txt, 'spec> {
    pub fn new(spec: &'spec DialectSpec, input: &'txt str) -> Self {
        Self {
            spec,
            input,
            cursor: 0,
            done: false,
            max_op_len: spec.max_op_len(),
        }
    }

    fn read_next(&mut self) -> Option<Token<'txt>> {
        if self.done {
            return None;
        }

        self.skip_ws_and_comments();

        self.read_eof()
            .or_else(|| self.read_char_token())
            .or_else(|| self.read_quoted_identifier())
            .or_else(|| self.read_string_single_quoted())
            .or_else(|| self.read_number_or_float())
            .or_else(|| self.read_keyword_or_identifier())
            .or_else(|| self.read_operator())
            .or_else(|| self.read_unknown())
    }

    fn read_eof(&mut self) -> Option<Token<'txt>> {
        if self.is_eof() {
            self.done = true;
            return Some(self.make_token_single_char(TokenKind::Eof));
        }
        None
    }

    fn read_char_token(&mut self) -> Option<Token<'txt>> {
        match self.peek() {
            Some('.') if !self.peek_nth(1).is_some_and(|c| c.is_ascii_digit()) => {
                Some(self.make_token_single_char(TokenKind::Dot))
            }
            Some(',') => Some(self.make_token_single_char(TokenKind::Comma)),
            Some('(') => Some(self.make_token_single_char(TokenKind::LeftParen)),
            Some(')') => Some(self.make_token_single_char(TokenKind::RightParen)),
            Some('[') => Some(self.make_token_single_char(TokenKind::LeftBracket)),
            Some(']') => Some(self.make_token_single_char(TokenKind::RightBracket)),
            Some(';') => Some(self.make_token_single_char(TokenKind::Semicolon)),
            _ => None,
        }
    }

    fn read_quoted_identifier(&mut self) -> Option<Token<'txt>> {
        let start = self.cursor;
        let style = QuoteStyle::from_open_char(self.peek()?)?;

        if !self.spec.supports_quote_style(style) {
            return None;
        }

        self.bump();
        self.consume_quoted(style.close_char());

        Some(self.make_token(start, TokenKind::IdentifierQuoted(style)))
    }

    fn read_string_single_quoted(&mut self) -> Option<Token<'txt>> {
        if self.peek()? != '\'' {
            return None;
        }

        let start = self.cursor;
        self.bump();
        self.consume_quoted('\'');

        Some(self.make_token(start, TokenKind::Str))
    }

    fn read_number_or_float(&mut self) -> Option<Token<'txt>> {
        let ch = self.peek()?;
        if !ch.is_ascii_digit() && ch != '.' {
            return None;
        }

        let start = self.cursor;
        let mut kind = TokenKind::Number;

        // leading digits or leading dot
        if ch == '.' {
            if !self.peek_nth(1).is_some_and(|c| c.is_ascii_digit()) {
                return None;
            }
            kind = TokenKind::Float;
            self.bump();
            self.consume_while(|c| c.is_ascii_digit());
        } else {
            self.consume_while(|c| c.is_ascii_digit());

            // fraction: .digits? (allow trailing dot or ".e" pattern)
            if self.peek() == Some('.') {
                let next = self.peek_nth(1);
                if next.is_none()
                    || !next.unwrap().is_ascii_alphabetic()
                    || next.is_some_and(|c| c == 'e' || c == 'E')
                {
                    kind = TokenKind::Float;
                    self.bump();
                    self.consume_while(|c| c.is_ascii_digit());
                }
            }
        }

        // exponent: e[+-]?digits
        if matches!(self.peek(), Some('e' | 'E')) {
            let next = self.peek_nth(1);
            let has_sign = next.is_some_and(|c| c == '+' || c == '-');
            let digit_offset = if has_sign { 2 } else { 1 };

            if self
                .peek_nth(digit_offset)
                .is_some_and(|c| c.is_ascii_digit())
            {
                kind = TokenKind::Float;
                self.bump();
                if has_sign {
                    self.bump();
                }
                self.consume_while(|c| c.is_ascii_digit());
            }
        }

        Some(self.make_token(start, kind))
    }

    fn read_operator(&mut self) -> Option<Token<'txt>> {
        let start = self.cursor;
        let remaining = &self.input[self.cursor..];
        let max_len = self.max_op_len;

        // Try longest match first (greedy)
        for len in (1..=max_len).rev() {
            if remaining.len() < len {
                continue;
            }
            if let Some(slice) = remaining.get(..len)
                && let Some(op) = self.spec.match_operator(slice) {
                    self.cursor += len;
                    return Some(self.make_token(start, TokenKind::Operator(op)));
                }
        }
        None
    }

    fn read_keyword_or_identifier(&mut self) -> Option<Token<'txt>> {
        if !DialectSpec::is_ident_start(self.peek()?) {
            return None;
        }

        let start = self.cursor;
        self.bump();
        self.consume_while(DialectSpec::is_ident_continue);

        let text = self.slice_from(start);
        let kind = self
            .spec
            .match_keyword(text)
            .map(TokenKind::Keyword)
            .or_else(|| self.spec.match_operator(text).map(TokenKind::Operator))
            .unwrap_or(TokenKind::Identifier);

        Some(self.make_token(start, kind))
    }

    fn read_unknown(&mut self) -> Option<Token<'txt>> {
        let start = self.cursor;
        self.bump();
        Some(self.make_token(start, TokenKind::Unknown))
    }

    fn consume_quoted(&mut self, close_quote: char) -> bool {
        while let Some(ch) = self.peek() {
            self.bump();
            if ch == close_quote {
                // Check for escaped quote (doubled)
                if self.peek() == Some(close_quote) {
                    self.bump();
                    continue;
                }
                return true;
            }
        }
        false
    }

    // ---------------- Skip irrelevant characters ----------------
    fn skip_ws_and_comments(&mut self) {
        loop {
            let before = self.cursor;
            self.skip_ws();
            if self.spec.supports_comment_style(CommentStyle::DoubleDash) {
                self.skip_line_comment();
            }
            if self.spec.supports_comment_style(CommentStyle::Hash) {
                self.skip_hash_comment();
            }
            if self.spec.supports_comment_style(CommentStyle::SlashStar) {
                self.skip_block_comment();
            }
            if self.cursor == before {
                break;
            }
        }
    }

    fn skip_ws(&mut self) {
        self.consume_while(|c| c.is_whitespace());
    }

    fn skip_line_comment(&mut self) {
        if self.starts_with("--") {
            self.consume_while(|c| c != '\n');
        }
    }

    fn skip_hash_comment(&mut self) {
        if self.peek() == Some('#') {
            self.consume_while(|c| c != '\n');
        }
    }

    fn skip_block_comment(&mut self) {
        if !self.starts_with("/*") {
            return;
        }
        self.cursor += 2;
        let mut depth = 1;
        while !self.is_eof() && depth > 0 {
            if self.starts_with("/*") {
                self.cursor += 2;
                depth += 1;
            } else if self.starts_with("*/") {
                self.cursor += 2;
                depth -= 1;
            } else {
                self.bump();
            }
        }
    }

    // ---------------- Token creation helpers ----------------
    #[inline]
    fn make_token(&self, start: usize, kind: TokenKind) -> Token<'txt> {
        Token {
            text: self.slice_from(start),
            kind,
            span: Span::new(start, self.cursor),
        }
    }

    #[inline]
    fn make_token_single_char(&mut self, kind: TokenKind) -> Token<'txt> {
        let start = self.cursor;
        self.bump();
        self.make_token(start, kind)
    }

    // ---------------- Reading utils ----------------
    #[inline]
    fn slice_from(&self, start: usize) -> &'txt str {
        &self.input[start..self.cursor]
    }

    #[inline]
    fn starts_with(&self, s: &str) -> bool {
        self.input[self.cursor..].starts_with(s)
    }

    #[inline]
    fn is_eof(&self) -> bool {
        self.cursor >= self.input.len()
    }

    #[inline]
    fn peek(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    #[inline]
    fn peek_nth(&self, n: usize) -> Option<char> {
        self.input[self.cursor..].chars().nth(n)
    }

    #[inline]
    fn bump(&mut self) -> Option<char> {
        if let Some(ch) = self.peek() {
            self.cursor += ch.len_utf8();
            Some(ch)
        } else {
            None
        }
    }

    #[inline]
    fn consume_while<F: Fn(char) -> bool>(&mut self, f: F) {
        while let Some(ch) = self.peek() {
            if f(ch) {
                self.bump();
            } else {
                break;
            }
        }
    }
}

impl<'txt, 'spec> Iterator for Lexer<'txt, 'spec> {
    type Item = Token<'txt>;
    fn next(&mut self) -> Option<Self::Item> {
        self.read_next()
    }
}
