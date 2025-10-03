use crate::{ast, span::Loc, token::TokenTape};

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
