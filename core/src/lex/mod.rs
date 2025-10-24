use crate::dialect::DialectSpec;

mod lexer;
mod rule;
mod tape;
mod types;

use lexer::Lexer;
pub use rule::*;
pub use tape::TokenTape;
pub use types::*;

pub fn lex<'txt>(spec: &DialectSpec, input: &'txt str) -> Vec<Token<'txt>> {
    Lexer::new(spec, input).collect()
}

pub fn token_tape<'txt, 'spec>(tokens: &'spec [Token<'txt>]) -> TokenTape<'txt, 'spec> {
    TokenTape::new(tokens)
}
