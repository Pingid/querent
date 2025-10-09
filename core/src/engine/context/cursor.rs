use crate::span::Span;
use crate::token::{Keyword, OpTag, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    /// The current position of the cursor.
    pub position: usize,
    /// The location of the cursor.
    pub location: Location,
    /// The text before the cursor. Empty if proceding a space.
    pub fragment: String,
    /// The span into text to replace.
    pub replace: Span,
    /// The keywords preceding the cursor.
    pub preceding: Vec<Keyword>,
    /// The current keyword token (if cursor is on/after a keyword)
    pub current_keyword: Option<Keyword>,
    /// For qualified names (e.g., table.column), the qualifier before the dot
    pub qualifier: Option<String>,
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
    Keyword,
    /// SELECT * FROM (^
    Paren,
    /// SELECT 1^
    Literal,
    /// SELECT a, ^
    Space(Box<Location>),
}

pub fn detect_cursor<'txt>(text: &'txt str, tokens: &[Token<'txt>], position: usize) -> Cursor {
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
        TokenKind::Keyword(_) => maybe_spaced(Location::Keyword),
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

    let preceding = tokens[..token_i]
        .iter()
        .filter_map(|t| match t.kind {
            TokenKind::Keyword(kw) => Some(kw),
            _ => None,
        })
        .collect();

    // Also track the current token's keyword if applicable
    let current_keyword = match token.kind {
        TokenKind::Keyword(kw) => Some(kw),
        _ => None,
    };

    // Extract qualifier if we're after a dot
    let qualifier = if matches!(token.kind, TokenKind::Dot) && token_i > 0 {
        // Look for the identifier before the dot
        let prev_token = &tokens[token_i - 1];
        match prev_token.kind {
            TokenKind::Identifier => Some(prev_token.text.to_string()),
            TokenKind::IdentifierQuoted(q) => Some(q.strip_quotes(prev_token.text).to_string()),
            _ => None,
        }
    } else {
        None
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
    use crate::{
        test_util::{ansi_tokens, with_caret_cursor},
        token::Keyword,
    };

    use super::*;

    #[test]
    fn start() {
        let text = ansi_detect_cursor("^");
        assert_eq!(text.location, Location::Start);
    }

    #[test]
    fn comma() {
        let text = ansi_detect_cursor("SELECT one,^ FROM users");
        assert_eq!(text.location, Location::Comma);
        assert_eq!(text.preceding, vec![Keyword::Select]);
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
        assert_eq!(text.location, Location::Operator(crate::token::OpTag::Add));
        let text = ansi_detect_cursor("SELECT one + ^ FROM users");
        assert_eq!(
            text.location,
            Location::Space(Box::new(Location::Operator(crate::token::OpTag::Add)))
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
        assert_eq!(text.location, Location::Keyword);
        let text = ansi_detect_cursor("SELECT ^ FROM users");
        assert_eq!(text.location, Location::Space(Box::new(Location::Keyword)));
    }

    #[test]
    fn quoted_ident() {
        let text = ansi_detect_cursor(r#"SELECT "one^" FROM users"#);
        assert_eq!(text.location, Location::Ident);
        assert_eq!(text.fragment, "one");
        assert_eq!(text.replace, Span::new(8, 11));
    }

    #[test]
    fn preceding() {
        let text = ansi_detect_cursor("SELECT * FROM users WHERE name = 'John'^");
        assert_eq!(
            text.preceding,
            vec![Keyword::Select, Keyword::From, Keyword::Where]
        );
    }

    fn ansi_detect_cursor(sql: &str) -> Cursor {
        let (text, pos) = with_caret_cursor(sql);
        let tokens = ansi_tokens(&text);
        detect_cursor(&text, &tokens, pos)
    }
}
