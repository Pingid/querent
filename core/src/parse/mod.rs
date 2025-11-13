use crate::ast;
use crate::lex::TokenKind;
use crate::lex::TokenTape;
use crate::span::Loc;

mod expr;

mod parser;
pub use parser::*;

pub fn parse_statement<'txt, 'tok>(
    tape: impl Into<TokenTape<'txt, 'tok>>,
) -> Option<Loc<ast::Statement>>
where
    'txt: 'tok,
{
    let mut parser = Parser::new(tape);
    parser.parse_statement()
}

pub fn parse_statement_at_cursor<'txt, 'tok>(
    tape: impl Into<TokenTape<'txt, 'tok>>, cursor: usize,
) -> Option<Loc<ast::Statement>>
where
    'txt: 'tok,
{
    let mut tape = tape.into();
    advance_to_statement_start(&mut tape, cursor);
    parse_statement(tape)
}

fn advance_to_statement_start<'txt, 'tok>(tape: &mut TokenTape<'txt, 'tok>, cursor: usize) {
    // Find the last semicolon before the cursor
    let mut last_semicolon_idx = None;

    for (idx, token) in tape.tokens.iter().enumerate() {
        // Stop if we've passed the cursor position
        if token.span.start >= cursor {
            break;
        }

        // Track the most recent semicolon
        if token.kind == TokenKind::Semicolon {
            last_semicolon_idx = Some(idx);
        }
    }

    // If we found a semicolon, advance to the token after it
    // Otherwise, go to the beginning (position 0)
    tape.pos = last_semicolon_idx.map(|idx| idx + 1).unwrap_or(0);
}
