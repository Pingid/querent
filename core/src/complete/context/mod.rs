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
use crate::dialect::DialectSpec;
use crate::lex::Token;
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
    pub scope: Scope<'a>,
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
}
