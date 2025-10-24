use std::ops::Deref;

use crate::ast;
use crate::dialect::DialectSpec;
use crate::lex::Token;
use crate::lex::TokenKind;
use crate::lex::lex;
use crate::parse::parse_statement_at_cursor;
use crate::schema;

mod cursor;
pub use cursor::*;

mod clause;
pub use clause::*;

mod scope;
pub use scope::*;

#[derive(Debug)]
pub struct Context<'a> {
    pub text: &'a str,
    pub tokens: Vec<Token<'a>>,
    pub schema: &'a schema::Cache,
    pub spec: &'a DialectSpec,
    pub cursor: Cursor<'a>,
    pub scope: ResolvedScope<'a>,
    pub clause: ClauseKind,
}

impl<'a> Context<'a> {
    pub fn build(
        spec: &'a DialectSpec, schema: &'a schema::Cache, text: &'a str, cursor: usize,
    ) -> Option<Self> {
        let tokens = lex(spec, &text);
        let Some(stmt) = parse_statement_at_cursor(&tokens, cursor) else {
            return None;
        };
        let cursor = detect_cursor(&text, &tokens, cursor);
        let scope = resolve_scope(&text, cursor.position, ast::Node::Statement(&stmt), schema);
        let clause = detect_clause_kind(&tokens, cursor.position);
        Some(Self {
            text,
            tokens,
            schema,
            spec,
            cursor,
            scope,
            clause,
        })
    }

    pub fn matches(&'a self) -> Match<'a> {
        Match::new(self)
    }
}

pub struct Match<'a>(&'a Context<'a>, bool);
impl<'a> Deref for Match<'a> {
    type Target = bool;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<'a> Match<'a> {
    pub fn new(ctx: &'a Context<'a>) -> Self {
        Self(ctx, false) // start from false when OR-ing conditions
    }

    pub fn clause(mut self, k: ClauseKind) -> Self {
        self.1 |= self.0.clause == k; // compare, not matches!
        self
    }

    pub fn loc<F: Fn(&Location) -> bool>(mut self, pred: F) -> Self {
        self.1 |= pred(&self.0.cursor.location);
        self
    }

    pub fn preceding<const N: usize>(mut self, pat: [TokenKind; N]) -> Self {
        self.1 |= self.0.cursor.preceding_matches(pat);
        self
    }

    pub fn preceding_slice(mut self, pat: &[TokenKind]) -> Self {
        let seq = &self.0.cursor.preceding;
        let n = pat.len();
        let ok = n <= seq.len() && &seq[seq.len() - n..] == pat;
        self.1 |= ok;
        self
    }
}
