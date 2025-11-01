use std::collections::HashSet;

use pat::Pat;
use pat::Predicate;
use pat::match_all;

use crate::lex::Keyword;
use crate::lex::OpTag;
use crate::lex::Operator;
use crate::lex::TokenKind;

#[derive(Debug, Clone, PartialEq)]
pub struct Rules(pub &'static [Rule]);

#[derive(Debug, Clone, PartialEq)]
pub struct Rule(pub Con, pub &'static [Next]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Next {
    Kw(Keyword),
    KwSeq(&'static [Keyword]),
    Seq(&'static [Next]),
    Query,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tok {
    Kw(Keyword),
    AnyOp,
    Op(OpTag),
    Kind(TokenKind),
    Debug,
}

impl Predicate for Tok {
    type Token = TokenKind;
    fn test(&self, token: &Self::Token) -> bool {
        test_kind(self, token)
    }
}

fn test_kind(tok: &Tok, token: &TokenKind) -> bool {
    match tok {
        Tok::Kw(kw) => token == &TokenKind::Keyword(*kw),
        Tok::Op(op) => match token {
            TokenKind::Operator(Operator { semantic_tag, .. }) => semantic_tag == op,
            _ => false,
        },
        Tok::Kind(kind) => token == kind,
        Tok::AnyOp => matches!(token, TokenKind::Operator(_)),
        Tok::Debug => {
            println!("match {:#?}", token);
            true
        }
    }
}

pub type Kw = Keyword;
pub type Op = OpTag;
pub type Tk = TokenKind;
pub type Con = Pat<'static, Tok>;

pub const fn debug() -> Con {
    Pat::Peek(&Pat::Atom(Tok::Debug))
}

pub const fn kw(kw: Keyword) -> Con {
    Pat::Atom(Tok::Kw(kw))
}

pub const fn op(op: OpTag) -> Con {
    Pat::Atom(Tok::Op(op))
}
pub const fn any_op() -> Con {
    Pat::Atom(Tok::AnyOp)
}

pub const fn tk(kind: TokenKind) -> Con {
    Pat::Atom(Tok::Kind(kind))
}

pub fn find_matches<'a>(
    rules: &'a [Rules], tokens: &'a [TokenKind],
) -> impl Iterator<Item = Next> + 'a {
    let t = match tokens.last() {
        Some(TokenKind::Eof) => &tokens[..tokens.len().saturating_sub(1)],
        _ => tokens,
    };
    let mut seen: HashSet<Next> = HashSet::new();
    rules
        .iter()
        .flat_map(|r| r.0)
        .filter(move |r| match_all::<true, _>(&r.0, t).is_ok())
        .flat_map(|r| r.1.iter().copied())
        .filter(move |then| seen.insert(*then))
}
