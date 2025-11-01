use crate::complete::context::ParsedStatement;
use crate::lex::Keyword;
use crate::lex::OpTag;
use crate::lex::Token;
use crate::lex::TokenKind;
use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor<'txt> {
    /// The current position of the cursor.
    pub position: usize,
    /// The location of the cursor.
    pub location: Location,
    /// The text before the cursor. Empty if proceding a space.
    pub fragment: String,
    /// The span into text to replace.
    pub replace: Span,
    /// The tokens from the start of the statement to the cursor.
    pub preceding: Vec<TokenKind>,
    /// The current keyword token (if cursor is on/after a keyword)
    pub current_keyword: Option<Keyword>,
    /// For qualified names (e.g., table.column), the qualifier before the dot
    pub qualifier: Option<Vec<&'txt str>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// ^
    Start,
    /// SELECT a,^
    Comma,
    /// SELECT a.^
    Dot,
    /// SELECT a +^
    Operator(OpTag),
    /// SELECT a^
    Ident,
    /// SELECT^
    Keyword(Keyword),
    /// SELECT * FROM (^
    Paren,
    /// SELECT 1^
    Literal,
    /// SELECT a, ^
    Space(Box<Location>),
}

impl<'txt> Cursor<'txt> {
    pub fn preceding_matches<const N: usize>(&self, other: [TokenKind; N]) -> bool {
        self.preceding[self.preceding.len().saturating_sub(N)..]
            .iter()
            .eq(other.iter())
    }
}

impl<'txt> From<&ParsedStatement<'txt>> for Cursor<'txt> {
    fn from(params: &ParsedStatement<'txt>) -> Self {
        detect_cursor(params.text, &params.tokens, params.cursor)
    }
}

pub fn detect_cursor<'txt>(
    text: &'txt str, tokens: &[Token<'txt>], position: usize,
) -> Cursor<'txt> {
    if tokens.len() <= 1 {
        return Cursor {
            position,
            location: Location::Start,
            fragment: String::new(),
            replace: Span::new(position, position),
            preceding: vec![],
            current_keyword: None,
            qualifier: None,
        };
    }

    let last_none_space_char = text[..position]
        .bytes()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(0, |i| i + 1);

    let after_space = last_none_space_char != position;
    let maybe_spaced = |loc| {
        if after_space {
            Location::Space(Box::new(loc))
        } else {
            loc
        }
    };

    let Some(token_i) = tokens
        .iter()
        .position(|t| t.span.contains_inclusive(last_none_space_char))
    else {
        return Cursor {
            position,
            location: Location::Start,
            fragment: String::new(),
            replace: Span::new(position, position),
            preceding: vec![],
            current_keyword: None,
            qualifier: None,
        };
    };

    let token = &tokens[token_i];

    let location = match token.kind {
        TokenKind::Comma => maybe_spaced(Location::Comma),
        TokenKind::Dot => maybe_spaced(Location::Dot),
        TokenKind::Operator(op) => maybe_spaced(Location::Operator(op.semantic_tag)),
        TokenKind::Keyword(kw) => maybe_spaced(Location::Keyword(kw)),
        TokenKind::Identifier | TokenKind::IdentifierQuoted(_) => maybe_spaced(Location::Ident),
        TokenKind::LeftParen => maybe_spaced(Location::Paren),
        TokenKind::Number | TokenKind::Float | TokenKind::Str => maybe_spaced(Location::Literal),
        _ => Location::Start,
    };

    let (fragment, replace) = if !after_space {
        match token.kind {
            TokenKind::Identifier => (token.text.to_string(), token.span),
            TokenKind::IdentifierQuoted(q) => (
                q.strip_quotes(token.text).to_string(),
                (token.span.start + 1..token.span.end - 1).into(),
            ),
            _ => (String::new(), Span::new(position, position)),
        }
    } else {
        (String::new(), Span::new(position, position))
    };

    let mut preceding = Vec::new();

    for t in tokens[..(token_i + 1).min(tokens.len() - 1)].iter().rev() {
        if let TokenKind::Semicolon = t.kind {
            break;
        }
        preceding.push(t.kind);
    }
    preceding.reverse();

    // Also track the current token's keyword if applicable
    let current_keyword = match token.kind {
        TokenKind::Keyword(kw) => Some(kw),
        _ => None,
    };

    // Extract qualifier if we're after a dot
    let qualifier = match matches!(token.kind, TokenKind::Dot) && token_i > 0 {
        false => None,
        true => {
            let mut i = token_i;
            let mut parts = Vec::new();
            while matches!(tokens[i].kind, TokenKind::Dot) && i > 0 {
                let prev_token = &tokens[i - 1];
                match prev_token.kind {
                    TokenKind::Identifier => parts.push(prev_token.text),
                    TokenKind::IdentifierQuoted(q) => parts.push(q.strip_quotes(prev_token.text)),
                    _ => break,
                }
                i = i.saturating_sub(2);
            }
            parts.reverse();
            Some(parts)
        }
    };

    Cursor {
        position,
        location,
        fragment,
        replace,
        preceding,
        current_keyword,
        qualifier,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::ansi_lex;
    use crate::test_utils::get_caret_cursor;

    #[test]
    fn start() {
        let text = ansi_detect_cursor("^");
        assert_eq!(text.location, Location::Start);
    }

    #[test]
    fn comma() {
        let text = ansi_detect_cursor("SELECT one,^ FROM users");
        assert_eq!(text.location, Location::Comma);
        let text = ansi_detect_cursor("SELECT one, ^ FROM users");
        assert_eq!(text.location, Location::Space(Box::new(Location::Comma)));
    }

    #[test]
    fn dot() {
        let text = ansi_detect_cursor("SELECT one.^ FROM users");
        assert_eq!(text.location, Location::Dot);
        let text = ansi_detect_cursor("SELECT one. ^ FROM users");
        assert_eq!(text.location, Location::Space(Box::new(Location::Dot)));
    }

    #[test]
    fn operator() {
        let text = ansi_detect_cursor("SELECT one +^ FROM users");
        assert_eq!(text.location, Location::Operator(crate::lex::OpTag::Add));
        let text = ansi_detect_cursor("SELECT one + ^ FROM users");
        assert_eq!(
            text.location,
            Location::Space(Box::new(Location::Operator(crate::lex::OpTag::Add)))
        );
    }

    #[test]
    fn ident() {
        let text = ansi_detect_cursor("SELECT one^ FROM users");
        assert_eq!(text.location, Location::Ident);
        assert_eq!(text.fragment, "one");
        assert_eq!(text.replace, Span::new(7, 10));
        let text = ansi_detect_cursor("SELECT one ^ FROM users");
        assert_eq!(text.location, Location::Space(Box::new(Location::Ident)));
        assert_eq!(text.fragment, "");
    }

    #[test]
    fn keyword() {
        let text = ansi_detect_cursor("SELECT^ FROM users");
        assert_eq!(text.location, Location::Keyword(Keyword::Select));
        let text = ansi_detect_cursor("SELECT ^ FROM users");
        assert_eq!(
            text.location,
            Location::Space(Box::new(Location::Keyword(Keyword::Select)))
        );
    }

    #[test]
    fn quoted_ident() {
        let text = ansi_detect_cursor(r#"SELECT "one^" FROM users"#);
        assert_eq!(text.location, Location::Ident);
        assert_eq!(text.fragment, "one");
        assert_eq!(text.replace, Span::new(8, 11));
    }

    fn ansi_detect_cursor(sql: &str) -> Cursor<'static> {
        let (text, pos) = get_caret_cursor(sql);
        let text_static: &'static str = Box::leak(text.to_string().into_boxed_str());
        let tokens = ansi_lex(text_static);
        detect_cursor(text_static, &tokens, pos)
    }
}
