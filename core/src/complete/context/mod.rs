use crate::{
    ast,
    dialect::DialectSpec,
    lex::{Token, lex},
    parse::parse_statement_at_cursor,
    schema,
};

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
    pub cursor: Cursor,
    pub scope: Scope,
    pub clause: ClauseKind,
}

impl<'a> Context<'a> {
    pub fn build(
        spec: &'a DialectSpec,
        schema: &'a schema::Cache,
        text: &'a str,
        cursor: usize,
    ) -> Option<Self> {
        let tokens = lex(spec, &text);
        let Some(stmt) = parse_statement_at_cursor(&tokens, cursor) else {
            return None;
        };
        let cursor = detect_cursor(&text, &tokens, cursor);
        let scope = build_scope(&text, cursor.position, ast::Node::Statement(&stmt));
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
