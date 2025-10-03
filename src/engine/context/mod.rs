use crate::{ast, token::Token};

mod cursor;
pub use cursor::*;

mod clause;
pub use clause::*;

mod scope;
pub use scope::*;

#[derive(Debug)]
pub struct Context {
    pub cursor: Cursor,
    pub scope: Scope,
    pub clause: ClauseKind,
}

pub fn build_context<'txt>(
    text: &'txt str,
    tokens: &'txt [Token<'txt>],
    position: usize,
    ast: impl Into<ast::Node<'txt>>,
) -> Context {
    let ast_node = ast.into();
    Context {
        cursor: detect_cursor(text, tokens, position),
        scope: build_scope(text, position, ast_node),
        clause: detect_clause_kind(tokens, position),
    }
}
