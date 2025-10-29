use crate::ast;
use crate::complete::context::ContextBuildParams;
use crate::complete::context::NamePath;
use crate::lex::Keyword;
use crate::lex::TokenKind;
use crate::span::Loc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause<'a> {
    pub kind: ClauseKind,
    pub pos: Option<ClausePosition>,
    pub func: Option<FunctionParam<'a>>,
    pub has_space: bool,
}

impl<'a> From<&ContextBuildParams<'a>> for Clause<'a> {
    fn from(params: &ContextBuildParams<'a>) -> Self {
        let kind = ClauseKind::from(params);
        let pos = <Option<ClausePosition>>::from(params);
        let func = <Option<FunctionParam<'a>>>::from(params);
        let has_space = params
            .proceeding_tokens()
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

impl<'a> From<&ContextBuildParams<'a>> for ClauseKind {
    fn from(params: &ContextBuildParams<'a>) -> Self {
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

impl<'a> From<&ContextBuildParams<'a>> for Option<FunctionParam<'a>> {
    fn from(params: &ContextBuildParams<'a>) -> Self {
        params.statement_node().find_map_rev(|node| {
            if !params.node_containing_cursor(&node) {
                return None;
            }
            match node {
                ast::Node::FunctionCallExpr(n) => {
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

impl<'a> From<&ContextBuildParams<'a>> for Option<ClausePosition> {
    fn from(params: &ContextBuildParams<'a>) -> Self {
        let prev = params.proceeding_tokens().next();
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
                ast::Node::BinaryExpr(binary) => {
                    if binary.op.is_some() {
                        let left_end = binary.left.span.end;
                        if params.cursor > left_end {
                            return Some(ClausePosition::ExprRight);
                        }
                    }
                    None
                }
                ast::Node::Projection(_) => Some(ClausePosition::ExprLeft),
                _ => None,
            }
        })
    }
}

impl<'a> ContextBuildParams<'a> {
    fn node_containing_cursor(&self, node: &ast::Node<'_>) -> bool {
        let eos = self.stmt.span.end;
        let is_end = node.span().end == eos && (self.cursor - 1) == eos;
        node.span().contains_inclusive(self.cursor) || is_end
    }

    fn statement_node(&self) -> ast::Node<'_> {
        ast::Node::Statement(&self.stmt)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ansi_detect_clause(sql: &str) -> Option<Clause<'static>> {
        let params = ContextBuildParams::new_ansi_static(sql)?;
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
