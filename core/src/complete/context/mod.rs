//! Context analysis for SQL code completion.
//!
//! This module provides the [`Context`] structure, which aggregates all the
//! information needed to generate accurate completion suggestions at a specific
//! cursor position in a SQL query.
//!
//! ## Overview
//!
//! The completion context combines multiple analysis layers:
//!
//! - **Lexical context** ([`Cursor`]): Information about the token at the
//!   cursor position, including its location, preceding tokens, and whether
//!   it's at the start/end of an identifier
//!
//! - **Syntactic context** ([`ClauseKind`]): Which SQL clause the cursor is
//!   located in (SELECT, FROM, WHERE, JOIN, etc.)
//!
//! - **Semantic context** ([`Scope`]): Which tables and columns are visible and
//!   accessible at the cursor position based on the query structure

use crate::ast;
use crate::complete::context::resolved::ResolvedScope;
use crate::dialect::DialectSpec;
use crate::lex::Token;
use crate::lex::lex;
use crate::parse::parse_statement_at_cursor;
use crate::schema;
use crate::span::Loc;

mod clause;
mod cursor;
mod resolved;
mod scope;

pub use clause::*;
pub use cursor::*;
pub use scope::*;

#[derive(Debug)]
pub struct Context<'a> {
    schema: &'a schema::Cache,
    spec: &'a DialectSpec,
    pub cursor: Cursor<'a>,
    pub scope: Scope<'a>,
    pub clause: Clause<'a>,
    pub resolved_scope: ResolvedScope<'a>,
}

impl<'a> Context<'a> {
    pub fn build(
        spec: &'a DialectSpec, schema: &'a schema::Cache, text: &'a str, cursor: usize,
    ) -> Option<Self> {
        let tokens = lex(spec, &text);
        let Some(stmt) = parse_statement_at_cursor(&tokens, cursor) else {
            return None;
        };
        let params = ParsedStatement {
            text,
            tokens,
            schema,
            spec,
            cursor,
            stmt,
        };
        let clause = Clause::from(&params);
        let cursor = Cursor::from(&params);
        let scope = Scope::from(&params);
        let resolved_scope = ResolvedScope::from(&params);
        Some(Self {
            schema: params.schema,
            spec: params.spec,
            cursor,
            scope,
            clause,
            resolved_scope,
        })
    }

    pub fn schema(&self) -> &'a schema::Cache {
        self.schema
    }

    pub fn spec(&self) -> &'a DialectSpec {
        self.spec
    }

    pub fn cursor(&self) -> &Cursor<'a> {
        &self.cursor
    }

    pub fn clause(&self) -> &Clause<'a> {
        &self.clause
    }

    pub fn resolved_scope(&self) -> &ResolvedScope<'a> {
        &self.resolved_scope
    }
}

#[derive(Debug)]
pub struct ParsedStatement<'a> {
    pub text: &'a str,
    pub tokens: Vec<Token<'a>>,
    pub schema: &'a schema::Cache,
    pub spec: &'a DialectSpec,
    pub cursor: usize,
    pub stmt: Loc<ast::Statement>,
}

#[cfg(test)]
impl ParsedStatement<'static> {
    pub fn new_ansi_static(text: &str) -> Option<Self> {
        use crate::test_util::ansi_tokens;
        use crate::test_util::get_leaky_static_caret_cursor;

        let (text, pos) = get_leaky_static_caret_cursor(text);
        let tokens = ansi_tokens(&text);
        let stmt = parse_statement_at_cursor(&tokens, pos)?;
        let schema = Box::leak(Box::new(schema::Cache::default()));
        Some(Self {
            text,
            tokens,
            schema,
            spec: &crate::dialect::ansi::SPEC,
            cursor: pos,
            stmt,
        })
    }
}
