use crate::ast;
use crate::ast::AstNode;
use crate::complete::context::NamePath;
use crate::complete::context::ParsedStatement;
use crate::lex::Keyword;
use crate::lex::Token;
use crate::lex::TokenKind;
use crate::span::Loc;
use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause<'a> {
    pub kind: ClauseKind,
    pub pos: Option<ClausePosition>,
    pub func: Option<FunctionParam<'a>>,
    pub has_space: bool,
}

impl<'a> From<&ParsedStatement<'a>> for Clause<'a> {
    fn from(params: &ParsedStatement<'a>) -> Self {
        let kind = ClauseKind::from(params);
        let pos = <Option<ClausePosition>>::from(params);
        let func = <Option<FunctionParam<'a>>>::from(params);
        let has_space = preceding_tokens(&params.tokens, params.cursor)
            .next()
            .map(|t| t.span.end < params.cursor)
            .unwrap_or(false);

        Clause {
            kind,
            pos,
            func,
            has_space,
        }
    }
}

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
    // /// SELECT * FROM users GROUP BY id HAVING ^
    // Having,
    /// SELECT * FROM users WINDOW ^
    Window,
    /// SELECT * FROM users ORDER BY ^
    OrderBy,
    /// SELECT * FROM users LIMIT ^
    Limit,
    /// SELECT * FROM users JOIN posts USING (^)
    Using,
}

impl<'a> From<&ParsedStatement<'a>> for ClauseKind {
    fn from(params: &ParsedStatement<'a>) -> Self {
        params
            .statement_node()
            .find_map_rev(|node| {
                if !params.node_containing_cursor(&node) {
                    return None;
                }
                match node {
                    ast::Node::Projection(_) => Some(ClauseKind::Select),
                    ast::Node::With(_) => Some(ClauseKind::With),
                    ast::Node::From(_) => Some(ClauseKind::From),
                    ast::Node::Where(_) => Some(ClauseKind::Where),
                    ast::Node::GroupBy(_) => Some(ClauseKind::GroupBy),
                    _ => None,
                }
            })
            .unwrap_or(ClauseKind::Statement)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionParam<'a> {
    pub name: NamePath<'a>,
    pub arg: usize,
}

impl<'a> From<&ParsedStatement<'a>> for Option<FunctionParam<'a>> {
    fn from(params: &ParsedStatement<'a>) -> Self {
        params.statement_node().find_map_rev(|node| {
            if !params.node_containing_cursor(&node) {
                return None;
            }
            match node {
                ast::Node::FunctionCall(n) => {
                    if !n.args.span.contains_inclusive(params.cursor) {
                        return None;
                    }
                    let arg = get_delimited_list_item_index(&n.args, params.cursor);
                    Some(FunctionParam {
                        name: NamePath::from_qualified_name(params.text, &n.name),
                        arg,
                    })
                }
                _ => None,
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClausePosition {
    /// SELECT^
    Keyword,
    /// "SELECT ^ " | "SELECT ab^ ";
    ExprLeft,
    /// SELECT a + ^ FROM items;
    ExprRight,
    /// SELECT price AS ^ FROM items;
    Alias,
}

impl<'a> From<&ParsedStatement<'a>> for Option<ClausePosition> {
    fn from(params: &ParsedStatement<'a>) -> Self {
        // preceding tokens
        let prev = preceding_tokens(&params.tokens, params.cursor).next();
        let exp = ast::Expr::find_where(params.statement_node(), |node| {
            params.node_containing_cursor(&node)
        });
        if let Some(exp) = exp {
            return match &exp.item {
                ast::Expr::Name(_) => Some(ClausePosition::ExprLeft),
                ast::Expr::Binary(bin) => match params.containing_cursor(bin.left.span) {
                    true => Some(ClausePosition::ExprLeft),
                    false => None,
                },
                exp => {
                    println!("\n\nexpr: {:#?}", exp);
                    None
                }
            };
        }
        params.statement_node().find_map_rev(|node| {
            if !params.node_containing_cursor(&node) {
                return None;
            }
            match node {
                ast::Node::ProjectionItem(item) => {
                    if let Some(alias) = &item.alias {
                        if params.cursor >= alias.span.start {
                            return Some(ClausePosition::Alias);
                        }
                    }
                    if let Some(prev) = prev
                        && prev.kind == TokenKind::Keyword(Keyword::As)
                    {
                        return Some(ClausePosition::Alias);
                    }
                    None
                }
                ast::Node::Binary(binary) => {
                    if binary.op.is_some() {
                        let left_end = binary.left.span.end;
                        if params.cursor > left_end {
                            return Some(ClausePosition::ExprRight);
                        }
                    }
                    None
                }
                ast::Node::Projection(_) => Some(ClausePosition::ExprLeft),
                ast::Node::Expr(expr) => match &expr.item {
                    ast::Expr::Empty => Some(ClausePosition::ExprLeft),
                    ast::Expr::Name(_) => Some(ClausePosition::ExprLeft),
                    exp => None,
                },
                _ => None,
            }
        })
    }
}

fn get_delimited_list_item_index<T>(list: &ast::DelimitedList<Loc<T>>, cursor: usize) -> usize {
    let mut index = 0;
    for (i, arg) in list.items.iter().enumerate() {
        let sep = list.seps.get(i).map(|s| s.span);
        if let Some(sep) = sep
            && cursor >= sep.end
        {
            index = i + 1;
        }
        if cursor < arg.span.start {
            return index;
        }
    }
    index
}

fn preceding_tokens<'a, 'b>(
    tokens: &'b [Token<'a>], cursor: usize,
) -> impl Iterator<Item = &'b Token<'a>> {
    tokens
        .iter()
        .rev()
        .filter(move |t| !matches!(t.kind, TokenKind::Eof) && t.span.end <= cursor)
        .take_while(move |t| cursor >= t.span.start)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn ansi_detect_clause(sql: &str) -> Option<Clause<'static>> {
        let params = ParsedStatement::new_ansi_static(sql)?;
        Some(Clause::from(&params))
    }

    macro_rules! assert_matches {
        ($text:expr, $got:expr, $pattern:pat $(if $guard:expr)? $(,)?) => {{
            assert!(
                matches!($got, $pattern $(if $guard)?),
                "\nexpected `{}` to match `{}`\n  got: {:?}",
                $text,
                stringify!($pattern),
                $got
            );
        }};
    }

    macro_rules! assert_clause {
        ($text:expr, $field:ident, $pattern:pat $(if $guard:expr)? $(,)?) => {{
            let clause = ansi_detect_clause($text);
            let field = clause.map(|c| c.$field);
            assert_matches!($text, field, Some($pattern) $(if $guard)?);
        }};
    }

    #[test]
    fn clause_kind() {
        use ClauseKind::*;
        assert_clause!("^", kind, Statement);
        assert_clause!("SEL^", kind, Statement);
        assert_clause!("WITH ^", kind, With);
        assert_clause!("SELECT^", kind, Select);
        assert_clause!("SELECT ^ FROM users", kind, Select);
        assert_clause!("SELECT * FROM (SELECT ^ FROM users) u", kind, Select);
        assert_clause!("SELECT * FROM^", kind, From);
        assert_clause!("SELECT * FROM ^", kind, From);
        assert_clause!("SELECT * FROM users WHERE^", kind, Where);
        assert_clause!("SELECT * FROM users WHERE ^", kind, Where);
        assert_clause!("SELECT * FROM users GROUP BY ^", kind, GroupBy);
        // assert_kind(
        //     ClauseKind::Having,
        //     "SELECT * FROM users GROUP BY id HAVING ^",
        // );
        // assert_kind(ClauseKind::Window, "SELECT * FROM users WINDOW ^");
        // assert_kind(ClauseKind::OrderBy, "SELECT * FROM users ORDER BY ^");
        // assert_kind(ClauseKind::Using, "SELECT * FROM users JOIN posts USING
        // ^");
    }

    #[test]
    fn clause_pos() {
        assert_clause!("SELECT ^", pos, Some(ClausePosition::ExprLeft));
        assert_clause!("SELECT foo^", pos, Some(ClausePosition::ExprLeft));
        assert_clause!("SELECT foo + ^", pos, Some(ClausePosition::ExprRight));
        assert_clause!("SELECT foo + 1^", pos, Some(ClausePosition::ExprRight));
        assert_clause!("SELECT foo AS ^", pos, Some(ClausePosition::Alias));
        assert_clause!("SELECT foo AS bar^", pos, Some(ClausePosition::Alias));
        assert_clause!("SELECT ab, ^", pos, Some(ClausePosition::ExprLeft));
        assert_clause!("SELECT COUNT(^)", pos, Some(ClausePosition::ExprLeft));
    }

    #[test]
    fn clause_func() {
        assert_clause!("SELECT COUNT(^)", func, Some(FunctionParam { arg: 0, .. }));
    }

    #[test]
    fn clause_has_space() {
        assert_clause!("SELECT ^", has_space, true);
        assert_clause!("SELECT foo^", has_space, false);
        assert_clause!("SELECT foo + ^", has_space, true);
        assert_clause!("SELECT foo + 1^", has_space, false);
        assert_clause!("SELECT foo AS ^", has_space, true);
        assert_clause!("SELECT foo AS bar^", has_space, false);
        assert_clause!("SELECT ab, ^", has_space, true);
    }
}
