use crate::lex::Keyword;
use crate::lex::Token;
use crate::lex::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseKind {
    /// SEL^
    Statement,
    /// WITH ^
    With,
    /// SELECT ^
    Select,
    /// SELECT * FROM ^
    From,
    /// SELECT * FROM users WHERE ^
    Where,
    /// SELECT * FROM users GROUP BY ^
    GroupBy,
    /// SELECT * FROM users GROUP BY id HAVING ^
    Having,
    /// SELECT * FROM users WINDOW ^
    Window,
    /// SELECT * FROM users ORDER BY ^
    OrderBy,
    /// SELECT * FROM users LIMIT ^
    Limit,
    /// SELECT * FROM users JOIN posts USING (^)
    Using,
}

pub fn detect_clause_kind<'txt>(tokens: &[Token<'txt>], position: usize) -> ClauseKind {
    tokens
        .iter()
        .take_while(|t| t.span.start <= position)
        .filter_map(|t| match t.kind {
            TokenKind::Keyword(Keyword::With) => Some(ClauseKind::With),
            TokenKind::Keyword(Keyword::Select) => Some(ClauseKind::Select),
            TokenKind::Keyword(Keyword::From) => Some(ClauseKind::From),
            TokenKind::Keyword(Keyword::Where) => Some(ClauseKind::Where),
            TokenKind::Keyword(Keyword::Group) => Some(ClauseKind::GroupBy),
            TokenKind::Keyword(Keyword::Having) => Some(ClauseKind::Having),
            TokenKind::Keyword(Keyword::Window) => Some(ClauseKind::Window),
            TokenKind::Keyword(Keyword::Order) => Some(ClauseKind::OrderBy),
            TokenKind::Keyword(Keyword::Limit) => Some(ClauseKind::Limit),
            TokenKind::Keyword(Keyword::Using) => Some(ClauseKind::Using),
            _ => None,
        })
        .last()
        .unwrap_or(ClauseKind::Statement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::ansi_tokens;
    use crate::test_util::get_caret_cursor;

    #[test]
    fn ansi_clause_detection() {
        assert_kind(ClauseKind::Statement, "^");
        assert_kind(ClauseKind::Statement, "SEL^");
        assert_kind(ClauseKind::With, "WITH^");
        assert_kind(ClauseKind::Select, "SELECT^");
        assert_kind(ClauseKind::Select, "SELECT ^ FROM users");
        assert_kind(ClauseKind::Select, "SELECT * FROM (SELECT ^ FROM users) u");
        assert_kind(ClauseKind::From, "SELECT * FROM^");
        assert_kind(ClauseKind::From, "SELECT * FROM ^");
        assert_kind(ClauseKind::Where, "SELECT * FROM users WHERE^");
        assert_kind(ClauseKind::Where, "SELECT * FROM users WHERE ^");
        assert_kind(ClauseKind::GroupBy, "SELECT * FROM users GROUP^");
        assert_kind(ClauseKind::GroupBy, "SELECT * FROM users GROUP BY ^");
        assert_kind(
            ClauseKind::Having,
            "SELECT * FROM users GROUP BY id HAVING^",
        );
        assert_kind(
            ClauseKind::Having,
            "SELECT * FROM users GROUP BY id HAVING ^",
        );
        assert_kind(ClauseKind::Window, "SELECT * FROM users WINDOW^");
        assert_kind(ClauseKind::Window, "SELECT * FROM users WINDOW ^");
        assert_kind(ClauseKind::OrderBy, "SELECT * FROM users ORDER^");
        assert_kind(ClauseKind::OrderBy, "SELECT * FROM users ORDER BY ^");
        assert_kind(ClauseKind::Using, "SELECT * FROM users JOIN posts USING^");
        assert_kind(
            ClauseKind::Using,
            "SELECT * FROM users JOIN posts USING (^)",
        );
    }

    fn assert_kind(exp: ClauseKind, sql: &str) {
        let kind = ansi_detect_clause_kind(sql);
        if kind != exp {
            println!("found: {:?},\nexp: {:?},\nsql: {}", kind, exp, sql);
        }
        assert_eq!(kind, exp);
    }

    fn ansi_detect_clause_kind(sql: &str) -> ClauseKind {
        let (text, pos) = get_caret_cursor(sql);
        let tokens = ansi_tokens(&text);
        detect_clause_kind(&tokens, pos)
    }
}
