use crate::dialect::DialectSpec;

mod lexer;
use lexer::Lexer;

mod tape;
pub use tape::TokenTape;

mod types;
pub use types::*;

pub fn lex<'txt>(spec: &DialectSpec, input: &'txt str) -> Vec<Token<'txt>> {
    Lexer::new(spec, input).collect()
}

pub fn token_tape<'txt, 'spec>(tokens: &'spec [Token<'txt>]) -> TokenTape<'txt, 'spec> {
    TokenTape::new(tokens)
}
