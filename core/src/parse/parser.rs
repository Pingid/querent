use crate::ast::*;
use crate::lex::{Keyword, OpTag, Token, TokenKind, TokenTape};
use crate::span::Loc;

#[derive(Debug)]
pub struct Parser<'txt, 'tok> {
    pub(crate) tokens: TokenTape<'txt, 'tok>,
}

impl<'txt, 'tok> Parser<'txt, 'tok> {
    pub fn new(tape: impl Into<TokenTape<'txt, 'tok>>) -> Self {
        Self {
            tokens: tape.into(),
        }
    }

    pub fn parse_statement(&mut self) -> Option<Loc<Statement>> {
        let _ = self.eat(TokenKind::Semicolon);
        let start = self.current_pos();
        if self.tokens.current().is_none() {
            let pos = self.tokens.prev().map(|t| t.span).unwrap_or(0.into());
            return Some(Loc::new(start, Statement::Partial(Loc::new(pos, ()))));
        }
        match self.tokens.current_kind() {
            Some(TokenKind::Identifier) if self.tokens.peek_kind() == Some(TokenKind::Eof) => {
                let sp = self.tokens.current().map(|t| t.span).unwrap_or(0.into());
                Some(Loc::new(start, Statement::Partial(Loc::new(sp, ()))))
            }
            Some(TokenKind::Eof) => Some(Loc::new(start, Statement::Partial(Loc::new(start, ())))),
            _ => self.node(|s| Some(Statement::Query(s.node(|s| s.parse_query())?))),
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

        let mut ctes: Vec<Loc<Cte>> = Vec::new();
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
                Some(CteMaterialization::Materialized)
            } else if self.eat_op_tag(OpTag::Not).is_some()
                && self.eat_kw(Keyword::Materialized).is_some()
            {
                Some(CteMaterialization::NotMaterialized)
            } else {
                None
            };

            // AS ( <query> )
            self.eat_kw(Keyword::As)?;
            self.eat(TokenKind::LeftParen)?;
            let query = Box::new(self.node(|s| s.parse_query())?);
            self.eat(TokenKind::RightParen);

            let cte = Cte {
                name,
                columns,
                materialized,
                query,
            };
            let span_start = cte.name.start;
            let span_end = self.prev_end(span_start);
            ctes.push(Loc::new((span_start, span_end), cte));

            if self.eat(TokenKind::Comma).is_some() {
                continue;
            }
            break;
        }

        Some(With { recursive, ctes })
    }

    fn parse_query_expr(&mut self) -> Option<QueryExpr> {
        let left = self.node(|s| s.parse_query_core())?;
        let set_ops = self.parse_many(|s| s.node(|s| s.parse_set_op_chain()));
        Some(QueryExpr { left, set_ops })
    }

    fn parse_set_op_chain(&mut self) -> Option<SetOpTerm> {
        let op = self.parse_set_op()?;
        let right = self.node(|s| s.parse_query_core())?;
        Some(SetOpTerm { op, right })
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

    fn parse_query_tail(&mut self) -> Option<QuerySuffix> {
        let mut order_by: Option<Loc<OrderBy>> = None;
        let mut limit: Option<Loc<Limit>> = None;
        let mut offset: Option<Loc<Offset>> = None;

        // Accept ORDER BY, LIMIT/FETCH, and OFFSET in any order
        loop {
            // Attempt ORDER BY if not set
            if order_by.is_none()
                && let Some(ob) = self.node(|s| s.parse_order_by())
            {
                order_by = Some(ob);
                continue;
            }

            // Attempt LIMIT/FETCH if not set
            if limit.is_none()
                && let Some(lim) = self.node(|s| s.parse_limit())
            {
                limit = Some(lim);
                continue;
            }

            // Attempt OFFSET if not set
            if offset.is_none()
                && let Some(off) = self.node(|s| s.parse_offset())
            {
                offset = Some(off);
                continue;
            }

            break;
        }

        if order_by.is_some() || limit.is_some() || offset.is_some() {
            Some(QuerySuffix {
                order_by,
                limit,
                offset,
            })
        } else {
            None
        }
    }

    fn parse_order_by(&mut self) -> Option<OrderBy> {
        self.eat_kws(&[Keyword::Order, Keyword::By])?;
        let items = self.comma_list1(|s| s.parse_order_by_item())?;
        Some(OrderBy { items })
    }

    fn parse_order_by_item(&mut self) -> Option<OrderByItem> {
        let expr = self.node(|s| s.parse_expr())?;
        let direction = self.parse_optional_enum(&[
            (Keyword::Asc, SortDirection::Asc),
            (Keyword::Desc, SortDirection::Desc),
        ]);
        Some(OrderByItem {
            expr,
            direction,
            nulls: None,
        })
    }

    fn parse_limit(&mut self) -> Option<Limit> {
        // Try FETCH FIRST syntax first
        if self.eat_kw(Keyword::Fetch).is_some() {
            self.eat_kw(Keyword::First)?;
            let count = self.node(|s| s.parse_expr())?;
            self.eat_kw(Keyword::Rows);
            self.eat_kw(Keyword::Only);
            return Some(Limit {
                count,
                style: LimitKind::FetchFirst,
            });
        }

        // Fall back to LIMIT syntax
        if self.eat_kw(Keyword::Limit).is_some() {
            let count = self.node(|s| s.parse_expr())?;
            return Some(Limit {
                count,
                style: LimitKind::Limit,
            });
        }

        None
    }

    fn parse_offset(&mut self) -> Option<Offset> {
        self.eat_kw(Keyword::Offset)?;
        let count = self.node(|s| s.parse_expr())?;
        let had_rows = self.eat_kw(Keyword::Rows).is_some(); // ROWS is optional
        Some(Offset {
            count,
            rows_keyword: had_rows,
        })
    }

    fn parse_query_core(&mut self) -> Option<QueryPrimary> {
        match self.tokens.current_kind() {
            Some(TokenKind::Keyword(Keyword::Select)) => {
                Some(QueryPrimary::Select(self.node(|s| s.parse_select_stmt())?))
            }
            _ => None,
        }
    }

    fn parse_select_stmt(&mut self) -> Option<Select> {
        self.eat_kw(Keyword::Select)?;
        Some(Select {
            distinct: self.parse_distinct().unwrap_or(SetQuantifier::All),
            projection: self.node(|s| s.parse_projection())?,
            from: self.node(|s| s.parse_from()),
            where_clause: self.clause_expr(Keyword::Where),
            group_by: self.node(|s| s.parse_group_by()),
            having: self.clause_expr(Keyword::Having),
            window: self.node(|s| s.parse_window_clause()),
            qualify: None,
        })
    }

    fn parse_projection(&mut self) -> Option<DelimitedList<Loc<ProjectionItem>>> {
        Some(
            self.comma_list1(|s| s.parse_select_item())
                .unwrap_or_default(),
        )
    }

    fn parse_select_item(&mut self) -> Option<ProjectionItem> {
        Some(ProjectionItem {
            expr: self.node(|s| s.parse_expr())?,
            alias: self.parse_alias(),
        })
    }

    fn parse_alias(&mut self) -> Option<Loc<SpannedStr>> {
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

    fn parse_from(&mut self) -> Option<From> {
        self.eat_kw(Keyword::From)?;
        Some(From {
            sources: self
                .comma_list1(|s| s.parse_table_ref())
                .unwrap_or_default(),
        })
    }

    fn parse_group_by(&mut self) -> Option<GroupBy> {
        self.eat_kws(&[Keyword::Group, Keyword::By])?;
        Some(GroupBy {
            items: self.comma_list1(|s| s.parse_group_by_item())?,
        })
    }

    fn parse_group_by_item(&mut self) -> Option<GroupByItem> {
        Some(GroupByItem::Expr(self.node(|s| s.parse_expr())?))
    }

    fn parse_window_clause(&mut self) -> Option<Window> {
        self.eat_kw(Keyword::Window)?;
        let mut windows = Vec::new();
        loop {
            // name
            let name = self.parse_ident()?;
            self.eat_kw(Keyword::As)?;
            self.eat(TokenKind::LeftParen)?;
            let spec = self.node(|s| s.parse_window_spec())?;
            self.eat(TokenKind::RightParen);
            windows.push(Loc::new(name, WindowDef { name, spec }));

            if self.eat(TokenKind::Comma).is_some() {
                continue;
            }
            break;
        }
        Some(Window { windows })
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

            left = TableRef::Join(Loc::new(
                (start, right_end),
                Join {
                    left: Box::new(Loc::new((start, right_end), left)),
                    kind,
                    right: Box::new(Loc::new((right_start, right_end), right)),
                    constraint,
                },
            ));
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
            return Some(TableRef::Factor(Loc::new(
                (start, end),
                TableFactor::Function(Loc::new(
                    (start, end),
                    FunctionTableFactor {
                        name,
                        args,
                        alias,
                        columns,
                        lateral,
                    },
                )),
            )));
        }

        // Plain named table
        let alias = self.parse_alias();
        let end = self.prev_end(start);
        Some(TableRef::Factor(Loc::new(
            (start, end),
            TableFactor::Named(Loc::new(
                (start, end),
                NamedTableFactor {
                    name,
                    alias,
                    lateral,
                },
            )),
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
            let end = self.prev_end(start);
            TableFactor::Subquery(Loc::new(
                (start, end),
                SubqueryTableFactor {
                    query,
                    alias,
                    lateral,
                },
            ))
        } else {
            // Parenthesized table ref
            let inner = Box::new(self.node(|s| s.parse_table_ref())?);
            self.eat(TokenKind::RightParen);
            TableFactor::Parenthesized(inner)
        };

        let end = self.prev_end(start);
        Some(TableRef::Factor(Loc::new((start, end), factor)))
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

    fn parse_join_constraint(&mut self) -> Option<Loc<JoinConstraint>> {
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

    fn parse_distinct(&mut self) -> Option<SetQuantifier> {
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
            return Some(SetQuantifier::DistinctOn(items));
        }

        Some(SetQuantifier::Distinct)
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
        // Allow * as a name part
        if matches!(self.tokens.current_kind(), Some(TokenKind::Operator(op)) if op.semantic_tag == OpTag::Mul)
        {
            self.tokens.advance()?;
            return Some(NamePart::Star);
        }
        // Allow non-reserved keywords as identifiers (e.g., "key", "value", but not "FROM", "WHERE")
        if let Some(TokenKind::Keyword(kw)) = self.tokens.current_kind()
            && !self.is_reserved_keyword(kw)
        {
            let span = self.tokens.current()?.span;
            self.tokens.advance()?;
            return Some(NamePart::Ident(span));
        }
        None
    }

    /// Check if a keyword is reserved and cannot be used as an identifier
    fn is_reserved_keyword(&self, kw: Keyword) -> bool {
        matches!(
            kw,
            Keyword::Select
                | Keyword::From
                | Keyword::Where
                | Keyword::Group
                | Keyword::Having
                | Keyword::Order
                | Keyword::Limit
                | Keyword::Offset
                | Keyword::Join
                | Keyword::Inner
                | Keyword::Left
                | Keyword::Right
                | Keyword::Full
                | Keyword::Cross
                | Keyword::On
                | Keyword::Using
                | Keyword::Union
                | Keyword::Intersect
                | Keyword::Except
                | Keyword::With
                | Keyword::As
                | Keyword::By
                | Keyword::Distinct
                | Keyword::All
        )
    }

    fn parse_ident(&mut self) -> Option<SpannedStr> {
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

        // BETWEEN is now an operator
        if self.tokens.current_operator_tag() != Some(OpTag::Between) {
            return None;
        }
        self.tokens.advance()?;

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
impl<'src, 'tok> Parser<'src, 'tok> {
    /// Parse a clause that starts with a keyword and contains an expression
    fn clause_expr(&mut self, kw: Keyword) -> Option<Loc<Expr>> {
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
    ) -> Option<DelimitedList<Loc<T>>> {
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
impl<'src, 'tok> Parser<'src, 'tok> {
    pub(crate) fn node<T>(&mut self, f: impl Fn(&mut Self) -> Option<T>) -> Option<Loc<T>> {
        let (result, span) = self.parse_with_span(|s| f(s))?;
        Some(Loc::new(span, result))
    }

    fn parse_with_span<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Option<T>,
    ) -> Option<(T, SpannedStr)> {
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
    ) -> Option<DelimitedList<Loc<T>>> {
        let mut list = DelimitedList::default();
        list.items.push(self.node(|s| parse_fn(s))?);

        while self.eat(sep).is_some() {
            list.seps.push(Loc::new(self.tokens.prev()?.span, sep));
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

    #[inline]
    pub(crate) fn eat_op_tag(&mut self, kw: OpTag) -> Option<&Token<'src>> {
        if matches!(self.tokens.current_kind(), Some(TokenKind::Operator(op)) if op.semantic_tag == kw)
        {
            self.tokens.advance()
        } else {
            None
        }
    }
}

impl<'src, 'tok> Iterator for Parser<'src, 'tok> {
    type Item = Loc<Statement>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.tokens.current_kind() == Some(TokenKind::Eof) {
            return None;
        }
        self.parse_statement()
    }
}
