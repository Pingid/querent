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
use crate::span::Loc;
use crate::span::Span;

mod clause;
mod cursor;
mod scope;

pub use clause::*;
pub use cursor::*;
pub use scope::*;

#[derive(Debug)]
pub struct Context<'a> {
    schema: &'a schema::Cache,
    spec: &'a DialectSpec,
    scope: Scope<'a>,
    cursor: Cursor<'a>,
    clause: Clause<'a>,
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
        let resolved_scope = Scope::from(&params);
        Some(Self {
            schema: params.schema,
            spec: params.spec,
            cursor,
            clause,
            scope: resolved_scope,
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

    pub fn scope(&self) -> &Scope<'a> {
        &self.scope
    }

    pub fn expected_data_type(&self) -> Option<schema::DataType> {
        let func = self.clause.func.as_ref()?;
        let func_name = func.name.to_string();
        let func_def = self.functions().find(|f| f.function_name() == func_name)?;
        func_def.parameter_types().get(func.arg).copied()
    }

    pub fn functions(&self) -> impl Iterator<Item = FunctionRef<'a>> {
        self.spec()
            .functions
            .values()
            .map(|func| FunctionRef::Spec(func))
            .chain(
                self.schema()
                    .get_functions()
                    .iter()
                    .map(|func| FunctionRef::Schema(func)),
            )
            .filter(|func| match (func.return_type(), self.clause.kind) {
                (schema::FuncReturnType::Scalar(_), ClauseKind::Select) => true,
                _ => false,
            })
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

impl<'a> ParsedStatement<'a> {
    pub fn node_containing_cursor(&self, node: &ast::Node<'_>) -> bool {
        self.containing_cursor(node.span())
    }

    pub fn containing_cursor(&self, span: impl Into<Span>) -> bool {
        let span = span.into();
        let eos = self.stmt.span.end;
        let is_end = span.end == eos && (self.cursor - 1) == eos;
        span.contains_inclusive(self.cursor) || is_end
    }

    pub fn ast_node(&self) -> ast::Node<'_> {
        ast::Node::Statement(&self.stmt)
    }
}

#[cfg(test)]
impl ParsedStatement<'static> {
    pub fn new_ansi_static(text: &str) -> Option<Self> {
        use crate::lex::lex;
        use crate::test_utils::leaky_static_caret_cursor;
        let (text, pos) = leaky_static_caret_cursor(text);
        let tokens = lex(&crate::dialect::ansi::SPEC, &text);
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
