use crate::tokenize::{Keyword, OpTag, Token, TokenKind, TokenTape};

use super::ast::*;

#[derive(Debug)]
pub struct Parser<'txt> {
    pub(crate) tokens: TokenTape<'txt>,
}

impl<'txt> Parser<'txt> {
    pub fn new(tokens: TokenTape<'txt>) -> Self {
        Self { tokens }
    }

    pub fn parse_statement(&mut self) -> Option<Statement> {
        let _ = self.eat(TokenKind::Semicolon);
        if self.tokens.current().is_none() {
            let pos = self.tokens.prev().map(|t| t.span).unwrap_or(0.into());
            return Some(Statement::Partial(node(pos, ())));
        }
        match self.tokens.current_kind() {
            Some(TokenKind::Identifier) if self.tokens.peek_kind() == Some(TokenKind::Eof) => {
                let sp = self.tokens.current().map(|t| t.span).unwrap_or(0.into());
                Some(Statement::Partial(node(sp, ())))
            }
            Some(TokenKind::Eof) => Some(Statement::Partial(node(0..0, ()))),
            _ => Some(Statement::Query(self.node(|s| s.parse_query())?)),
        }
    }

    pub(crate) fn parse_query(&mut self) -> Option<Query> {
        Some(Query {
            with: self.node(|s| s.parse_with()),
            body: self.node(|s| s.parse_query_expr()),
            tail: self.node(|s| s.parse_query_tail()),
        })
    }

    fn parse_with(&mut self) -> Option<With> {
        // WITH [RECURSIVE] cte [, cte ...]
        self.eat_kw(Keyword::With)?;
        let recursive = self.eat_kw(Keyword::Recursive).is_some();

        let mut ctes: Vec<Node<CTE>> = Vec::new();
        loop {
            // CTE name
            let name = self.parse_ident()?;

            // Optional column list
            let columns = if self.eat(TokenKind::LeftParen).is_some() {
                let cols = if self.tokens.current_kind() == Some(TokenKind::RightParen) {
                    DelimitedList::default()
                } else {
                    self.parse_list1(TokenKind::Comma, |s| s.parse_ident())?
                };
                self.eat(TokenKind::RightParen);
                Some(cols)
            } else {
                None
            };

            // Optional MATERIALIZED / NOT MATERIALIZED
            let materialized = if self.eat_kw(Keyword::Materialized).is_some() {
                Some(Materialized::Materialized)
            } else if self.eat_kw(Keyword::Not).is_some()
                && self.eat_kw(Keyword::Materialized).is_some()
            {
                Some(Materialized::NotMaterialized)
            } else {
                None
            };

            // AS ( <query> )
            self.eat_kw(Keyword::As)?;
            self.eat(TokenKind::LeftParen)?;
            let query = Box::new(self.node(|s| s.parse_query())?);
            self.eat(TokenKind::RightParen);

            let cte = CTE {
                name,
                columns,
                materialized,
                query,
            };
            let span_start = cte.name.start;
            let span_end = self.prev_end(span_start);
            ctes.push(Node::new((span_start, span_end), cte));

            if self.eat(TokenKind::Comma).is_some() {
                continue;
            }
            break;
        }

        Some(With { recursive, ctes })
    }

    fn parse_query_expr(&mut self) -> Option<QueryExpr> {
        let left = self.node(|s| s.parse_query_core())?;
        let set_ops = self.parse_many(|s| s.parse_set_op_chain());
        Some(QueryExpr { left, set_ops })
    }

    fn parse_set_op_chain(&mut self) -> Option<SetOpChain> {
        let op = self.parse_set_op()?;
        let right = self.node(|s| s.parse_query_core())?;
        Some(SetOpChain { op, right })
    }

    fn parse_set_op(&mut self) -> Option<SetOp> {
        let kw = match self.tokens.current_kind()? {
            TokenKind::Keyword(
                kw @ (Keyword::Union | Keyword::Intersect | Keyword::Except | Keyword::Minus),
            ) => kw,
            _ => return None,
        };
        self.tokens.advance();
        let all = self.eat_kw(Keyword::All).is_some();

        Some(match kw {
            Keyword::Union => SetOp::Union { all },
            Keyword::Intersect => SetOp::Intersect { all },
            Keyword::Except => SetOp::Except { all },
            Keyword::Minus => SetOp::Minus { all },
            _ => unreachable!(),
        })
    }

    fn parse_query_tail(&mut self) -> Option<QueryTail> {
        let mut order_by: Option<Node<OrderByClause>> = None;
        let mut limit: Option<Node<LimitClause>> = None;
        let mut offset: Option<Node<OffsetClause>> = None;

        // Accept ORDER BY, LIMIT/FETCH, and OFFSET in any order
        loop {
            // Attempt ORDER BY if not set
            if order_by.is_none() {
                if let Some(ob) = self.node(|s| s.parse_order_by()) {
                    order_by = Some(ob);
                    continue;
                }
            }

            // Attempt LIMIT/FETCH if not set
            if limit.is_none() {
                if let Some(lim) = self.node(|s| s.parse_limit()) {
                    limit = Some(lim);
                    continue;
                }
            }

            // Attempt OFFSET if not set
            if offset.is_none() {
                if let Some(off) = self.node(|s| s.parse_offset()) {
                    offset = Some(off);
                    continue;
                }
            }

            break;
        }

        if order_by.is_some() || limit.is_some() || offset.is_some() {
            Some(QueryTail {
                order_by,
                limit,
                offset,
            })
        } else {
            None
        }
    }

    fn parse_order_by(&mut self) -> Option<OrderByClause> {
        self.eat_kws(&[Keyword::Order, Keyword::By])?;
        let items = self.comma_list1(|s| s.parse_order_by_item())?;
        Some(OrderByClause { items })
    }

    fn parse_order_by_item(&mut self) -> Option<OrderByItem> {
        let expr = self.node(|s| s.parse_expr())?;
        let direction = self.parse_optional_enum(&[
            (Keyword::Asc, OrderDirection::Asc),
            (Keyword::Desc, OrderDirection::Desc),
        ]);
        Some(OrderByItem {
            expr,
            direction,
            nulls: None,
        })
    }

    fn parse_limit(&mut self) -> Option<LimitClause> {
        // Try FETCH FIRST syntax first
        if self.eat_kw(Keyword::Fetch).is_some() {
            self.eat_kw(Keyword::First)?;
            let count = self.node(|s| s.parse_expr())?;
            self.eat_kw(Keyword::Rows);
            self.eat_kw(Keyword::Only);
            return Some(LimitClause {
                count,
                style: LimitStyle::FetchFirst,
            });
        }

        // Fall back to LIMIT syntax
        if self.eat_kw(Keyword::Limit).is_some() {
            let count = self.node(|s| s.parse_expr())?;
            return Some(LimitClause {
                count,
                style: LimitStyle::Limit,
            });
        }

        None
    }

    fn parse_offset(&mut self) -> Option<OffsetClause> {
        self.eat_kw(Keyword::Offset)?;
        let count = self.node(|s| s.parse_expr())?;
        let had_rows = self.eat_kw(Keyword::Rows).is_some(); // ROWS is optional
        Some(OffsetClause {
            count,
            rows_keyword: had_rows,
        })
    }

    fn parse_query_core(&mut self) -> Option<QueryCore> {
        match self.tokens.current_kind() {
            Some(TokenKind::Keyword(Keyword::Select)) => {
                Some(QueryCore::Select(self.parse_select_stmt()?))
            }
            _ => None,
        }
    }

    fn parse_select_stmt(&mut self) -> Option<SelectStmt> {
        self.eat_kw(Keyword::Select)?;
        Some(SelectStmt {
            distinct: self.parse_distinct().unwrap_or(Distinct::All),
            projection: self.node(|s| s.parse_projection())?,
            from: self.node(|s| s.parse_from()),
            where_clause: self.clause_expr(Keyword::Where),
            group_by: self.node(|s| s.parse_group_by()),
            having: self.clause_expr(Keyword::Having),
            window: self.node(|s| s.parse_window_clause()),
            qualify: None,
        })
    }

    fn parse_projection(&mut self) -> Option<DelimitedList<Node<SelectItem>>> {
        Some(
            self.comma_list1(|s| s.parse_select_item())
                .unwrap_or_default(),
        )
    }

    fn parse_select_item(&mut self) -> Option<SelectItem> {
        Some(SelectItem {
            expr: self.node(|s| s.parse_expr())?,
            alias: self.parse_alias(),
        })
    }

    fn parse_alias(&mut self) -> Option<Node<Ident>> {
        if self.eat_kw(Keyword::As).is_some() {
            return self.node(|s| s.parse_ident());
        }

        // Try to parse identifier as alias (without AS keyword)
        match self.tokens.current_kind() {
            Some(TokenKind::Identifier | TokenKind::IdentifierQuoted(_)) => {
                self.node(|s| s.parse_ident())
            }
            _ => None,
        }
    }

    fn parse_from(&mut self) -> Option<FromClause> {
        self.eat_kw(Keyword::From)?;
        Some(FromClause {
            sources: self
                .comma_list1(|s| s.parse_table_ref())
                .unwrap_or_default(),
        })
    }

    fn parse_group_by(&mut self) -> Option<GroupByClause> {
        self.eat_kws(&[Keyword::Group, Keyword::By])?;
        Some(GroupByClause {
            items: self.comma_list1(|s| s.parse_group_by_item())?,
        })
    }

    fn parse_group_by_item(&mut self) -> Option<GroupByItem> {
        Some(GroupByItem::Expr(self.node(|s| s.parse_expr())?))
    }

    fn parse_window_clause(&mut self) -> Option<WindowClause> {
        self.eat_kw(Keyword::Window)?;
        let mut windows = Vec::new();
        loop {
            // name
            let name = self.parse_ident()?;
            self.eat_kw(Keyword::As)?;
            self.eat(TokenKind::LeftParen)?;
            let spec = self.node(|s| s.parse_window_spec())?;
            self.eat(TokenKind::RightParen);
            windows.push(Node::new(name, WindowDef { name, spec }));

            if self.eat(TokenKind::Comma).is_some() {
                continue;
            }
            break;
        }
        Some(WindowClause { windows })
    }

    fn parse_table_ref(&mut self) -> Option<TableRef> {
        let start = self.current_pos();
        let mut left = self.parse_table_factor()?;

        // Build left-associative join tree
        while let Some(kind) = self.parse_join_kind() {
            let right_start = self.current_pos();
            let right = self.parse_table_factor()?;
            let constraint = self.parse_join_constraint();
            let right_end = self.prev_end(right_start);

            left = TableRef::Join {
                left: Box::new(Node::new((start, right_end), left)),
                kind,
                right: Box::new(Node::new((right_start, right_end), right)),
                constraint,
            };
        }

        Some(left)
    }

    fn parse_table_factor(&mut self) -> Option<TableRef> {
        let start = self.current_pos();
        let lateral = self.eat_kw(Keyword::Lateral).is_some();

        // Parenthesized subquery or table ref
        if self.eat(TokenKind::LeftParen).is_some() {
            return self.parse_paren_table_factor(start, lateral);
        }

        // Named table or function call
        let name = self.node(|s| s.parse_qname())?;

        // Function call if followed by '('
        if self.eat(TokenKind::LeftParen).is_some() {
            let args = if self.tokens.current_kind() == Some(TokenKind::RightParen) {
                DelimitedList::default()
            } else {
                self.parse_list1(TokenKind::Comma, |s| s.parse_expr())?
            };
            self.eat(TokenKind::RightParen);

            // Optional alias
            let alias = self.parse_alias();

            // Optional column alias list: (col1, col2, ...)
            let columns = if self.eat(TokenKind::LeftParen).is_some() {
                let cols = if self.tokens.current_kind() == Some(TokenKind::RightParen) {
                    DelimitedList::default()
                } else {
                    // Accept identifiers or keywords as column aliases
                    self.parse_list1(TokenKind::Comma, |s| match s.tokens.current_kind()? {
                        TokenKind::Identifier | TokenKind::IdentifierQuoted(_) => s.parse_ident(),
                        TokenKind::Keyword(_) => {
                            let span = s.tokens.current()?.span;
                            s.tokens.advance()?;
                            Some(span)
                        }
                        _ => None,
                    })?
                };
                self.eat(TokenKind::RightParen);
                Some(cols)
            } else {
                None
            };

            let end = self.prev_end(start);
            return Some(TableRef::Factor(Node::new(
                (start, end),
                TableFactor::Function {
                    name,
                    args,
                    alias,
                    columns,
                    lateral,
                },
            )));
        }

        // Plain named table
        let alias = self.parse_alias();
        let end = self.prev_end(start);
        Some(TableRef::Factor(Node::new(
            (start, end),
            TableFactor::Named {
                name,
                alias,
                lateral,
            },
        )))
    }

    fn parse_paren_table_factor(&mut self, start: usize, lateral: bool) -> Option<TableRef> {
        // Check if it's a subquery (starts with SELECT, WITH)
        let factor = if matches!(
            self.tokens.current_kind(),
            Some(TokenKind::Keyword(Keyword::Select | Keyword::With))
        ) {
            // Subquery
            let query = self.node(|s| s.parse_query())?;
            self.eat(TokenKind::RightParen);
            let alias = self.parse_alias();
            TableFactor::Subquery {
                query,
                alias,
                lateral,
            }
        } else {
            // Parenthesized table ref
            let inner = Box::new(self.node(|s| s.parse_table_ref())?);
            self.eat(TokenKind::RightParen);
            TableFactor::Parenthesized { inner }
        };

        let end = self.prev_end(start);
        Some(TableRef::Factor(Node::new((start, end), factor)))
    }

    fn parse_join_kind(&mut self) -> Option<JoinKind> {
        let natural = self.eat_kw(Keyword::Natural).is_some();

        // Parse join type
        let (base, outer) = match self.tokens.current_kind()? {
            TokenKind::Keyword(Keyword::Inner) => {
                self.tokens.advance();
                self.eat_kw(Keyword::Join)?;
                (JoinBase::Inner, false)
            }
            TokenKind::Keyword(kw @ (Keyword::Left | Keyword::Right | Keyword::Full)) => {
                self.tokens.advance();
                let outer = self.eat_kw(Keyword::Outer).is_some();
                self.eat_kw(Keyword::Join)?;
                let base = match kw {
                    Keyword::Left => JoinBase::Left,
                    Keyword::Right => JoinBase::Right,
                    Keyword::Full => JoinBase::Full,
                    _ => unreachable!(),
                };
                (base, outer)
            }
            TokenKind::Keyword(Keyword::Cross) => {
                self.tokens.advance();
                self.eat_kw(Keyword::Join)?;
                (JoinBase::Cross, false)
            }
            TokenKind::Keyword(Keyword::Join) => {
                self.tokens.advance();
                (JoinBase::Inner, false)
            }
            _ => return None,
        };

        Some(JoinKind {
            base,
            outer,
            natural,
        })
    }

    fn parse_join_constraint(&mut self) -> Option<Node<JoinConstraint>> {
        self.node(|s| {
            if s.eat_kw(Keyword::On).is_some() {
                return Some(JoinConstraint::On(s.node(|s2| s2.parse_expr())?));
            }

            if s.eat_kw(Keyword::Using).is_some() {
                s.eat(TokenKind::LeftParen)?;
                let columns = s.comma_list1(|s2| s2.parse_ident())?;
                s.eat(TokenKind::RightParen);
                return Some(JoinConstraint::Using(columns));
            }

            None
        })
    }

    fn parse_distinct(&mut self) -> Option<Distinct> {
        let _ = self.eat_kw(Keyword::Distinct)?;

        // Check for optional ON ( ... ) list
        if self.eat_kw(Keyword::On).is_some() {
            // Parse parenthesized expression list
            self.eat(TokenKind::LeftParen)?;
            let items = if self.tokens.current_kind() == Some(TokenKind::RightParen) {
                // Empty list tolerated (partial input)
                DelimitedList::default()
            } else {
                self.parse_list1(TokenKind::Comma, |s| s.parse_expr())?
            };
            self.eat(TokenKind::RightParen);
            return Some(Distinct::DistinctOn(items));
        }

        Some(Distinct::Distinct)
    }

    pub(crate) fn parse_qname(&mut self) -> Option<QualifiedName> {
        Some(QualifiedName {
            parts: self.parse_list1(TokenKind::Dot, |s| s.parse_name_part())?,
        })
    }

    pub(crate) fn parse_name_part(&mut self) -> Option<NamePart> {
        if let Some(span) = self.parse_ident() {
            return Some(NamePart::Ident(span));
        }
        if self.tokens.current_operator_tag() == Some(OpTag::Mul) {
            self.tokens.advance()?;
            return Some(NamePart::Star);
        }
        None
    }

    fn parse_ident(&mut self) -> Option<Ident> {
        match self.tokens.current_kind()? {
            TokenKind::Identifier | TokenKind::IdentifierQuoted(_) => {
                let span = self.tokens.current()?.span;
                self.tokens.advance()?;
                Some(span)
            }
            _ => None,
        }
    }

    pub(crate) fn parse_window_spec(&mut self) -> Option<WindowSpec> {
        let partition_by = if self.eat_kw(Keyword::Partition).is_some() {
            self.eat_kw(Keyword::By)?;
            Some(self.comma_list1(|s| s.parse_expr())?)
        } else {
            None
        };

        Some(WindowSpec {
            partition_by,
            order_by: self.node(|s| s.parse_order_by()),
            frame: self.node(|s| s.parse_window_frame()).map(Box::new),
        })
    }

    fn parse_window_frame(&mut self) -> Option<WindowFrame> {
        let unit = self.parse_optional_enum(&[
            (Keyword::Rows, FrameUnit::Rows),
            (Keyword::Range, FrameUnit::Range),
        ])?;

        self.eat_kw(Keyword::Between)?;
        let start = Box::new(self.node(|s| s.parse_frame_bound())?);

        // Skip AND operator
        if self.tokens.current_operator_tag() == Some(OpTag::And) {
            self.tokens.advance();
        }

        let end = Some(Box::new(self.node(|s| s.parse_frame_bound())?));

        Some(WindowFrame { unit, start, end })
    }

    fn parse_frame_bound(&mut self) -> Option<FrameBound> {
        match self.tokens.current_kind()? {
            TokenKind::Keyword(Keyword::Unbounded) => {
                self.tokens.advance()?;
                if self.eat_kw(Keyword::Preceding).is_some() {
                    Some(FrameBound::UnboundedPreceding)
                } else {
                    self.eat_kw(Keyword::Following)?;
                    Some(FrameBound::UnboundedFollowing)
                }
            }
            TokenKind::Keyword(Keyword::Current) => {
                self.tokens.advance()?;
                self.eat_kw(Keyword::Row)?;
                Some(FrameBound::CurrentRow)
            }
            _ => {
                let expr = Box::new(self.node(|s| s.parse_expr())?);
                if self.eat_kw(Keyword::Preceding).is_some() {
                    Some(FrameBound::Preceding(expr))
                } else {
                    self.eat_kw(Keyword::Following)?;
                    Some(FrameBound::Following(expr))
                }
            }
        }
    }
}

// ---------------- Utility Methods ----------------
impl<'src> Parser<'src> {
    /// Parse a clause that starts with a keyword and contains an expression
    fn clause_expr(&mut self, kw: Keyword) -> Option<Node<Expr>> {
        self.eat_kw(kw)?;
        self.node(|s| Some(s.parse_expr().unwrap_or(Expr::Empty)))
    }

    /// Parse zero or more items using a parser function
    fn parse_many<T>(&mut self, mut parse_fn: impl FnMut(&mut Self) -> Option<T>) -> Vec<T> {
        let mut items = Vec::new();
        while let Some(item) = parse_fn(self) {
            items.push(item);
        }
        items
    }

    /// Parse a comma-separated list (at least one item)
    fn comma_list1<T>(
        &mut self,
        parse_fn: impl Fn(&mut Self) -> Option<T>,
    ) -> Option<DelimitedList<Node<T>>> {
        self.parse_list1(TokenKind::Comma, parse_fn)
    }

    /// Expect a sequence of keywords in order
    fn eat_kws(&mut self, keywords: &[Keyword]) -> Option<()> {
        for &kw in keywords {
            self.eat_kw(kw)?;
        }
        Some(())
    }

    /// Parse an optional enum value based on keyword
    fn parse_optional_enum<T: Copy>(&mut self, mappings: &[(Keyword, T)]) -> Option<T> {
        for &(kw, value) in mappings {
            if self.eat_kw(kw).is_some() {
                return Some(value);
            }
        }
        None
    }

    /// Get current position, defaulting to 0
    #[inline]
    fn current_pos(&mut self) -> usize {
        self.tokens.current().map(|t| t.span.start).unwrap_or(0)
    }

    /// Get previous token end position, defaulting to start
    #[inline]
    fn prev_end(&mut self, default: usize) -> usize {
        self.tokens.prev().map(|t| t.span.end).unwrap_or(default)
    }
}

// ---------------- Core Combinators ----------------
impl<'src> Parser<'src> {
    pub(crate) fn node<T>(&mut self, f: impl Fn(&mut Self) -> Option<T>) -> Option<Node<T>> {
        let (result, span) = self.parse_with_span(|s| f(s))?;
        Some(Node::new(span, result))
    }

    fn parse_with_span<T>(&mut self, f: impl FnOnce(&mut Self) -> Option<T>) -> Option<(T, Ident)> {
        let start = self.current_pos();
        let result = f(self)?;
        let end = self.prev_end(start);
        Some((result, (start, end).into()))
    }

    /// Parse a delimited list with at least one item (tolerates trailing separator)
    pub(crate) fn parse_list1<T>(
        &mut self,
        sep: TokenKind,
        parse_fn: impl Fn(&mut Self) -> Option<T>,
    ) -> Option<DelimitedList<Node<T>>> {
        let mut list = DelimitedList::default();
        list.items.push(self.node(|s| parse_fn(s))?);

        while self.eat(sep).is_some() {
            list.seps.push(Node::new(self.tokens.prev()?.span, sep));
            if let Some(item) = self.node(|s| parse_fn(s)) {
                list.items.push(item);
            } else {
                break;
            }
        }
        Some(list)
    }

    #[inline]
    pub(crate) fn eat(&mut self, k: TokenKind) -> Option<&Token<'src>> {
        if self.tokens.is_at(k) {
            self.tokens.advance()
        } else {
            None
        }
    }

    #[inline]
    pub(crate) fn eat_kw(&mut self, kw: Keyword) -> Option<&Token<'src>> {
        self.eat(TokenKind::Keyword(kw))
    }
}

impl<'src> Iterator for Parser<'src> {
    type Item = Statement;
    fn next(&mut self) -> Option<Self::Item> {
        if self.tokens.current_kind() == Some(TokenKind::Eof) {
            return None;
        }
        let stmt = self.parse_statement();
        eprintln!(
            "Parsing statement {} {:?} {:?}",
            &self.tokens.pos,
            &self.tokens.current(),
            stmt
        );
        stmt
    }
}
