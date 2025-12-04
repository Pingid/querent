#![allow(dead_code)]

use crate::ast::*;
use crate::dialect::DialectSpec;
use crate::lex::Assoc;
use crate::lex::Fixity;
use crate::lex::Keyword as KW;
use crate::lex::OpTag;
use crate::lex::Token;
use crate::lex::TokenKind as TK;
use crate::span::Loc;
use crate::span::Span;

type R<T> = Option<T>;

pub struct WinnowParser<'txt, 'tok> {
    tokens: &'tok [Token<'txt>],
    pos: usize,
    spec: &'tok DialectSpec,
}

impl<'txt, 'tok> WinnowParser<'txt, 'tok> {
    pub fn new(tokens: &'tok [Token<'txt>], spec: &'tok DialectSpec) -> Self {
        Self {
            tokens,
            pos: 0,
            spec,
        }
    }

    pub fn parse_statement(&mut self) -> R<Loc<Statement>> {
        while self.eat(TK::Semicolon).is_some() {}
        let start = self.pos();
        match self.kind() {
            None | Some(TK::Eof) => {
                return Some(loc(start, Statement::Partial(loc(self.span(), ()))));
            }
            Some(TK::Identifier) if matches!(self.peek_kind(), Some(TK::Eof) | None) => {
                return Some(loc(start, Statement::Partial(loc(self.span(), ()))));
            }
            Some(TK::Keyword(KW::Insert)) => {
                let insert = self.spanned(Self::parse_insert)?;
                return Some(loc((start, insert.span.end), Statement::Insert(insert)));
            }
            Some(TK::Keyword(KW::Update)) => {
                let update = self.spanned(Self::parse_update)?;
                return Some(loc((start, update.span.end), Statement::Update(update)));
            }
            Some(TK::Keyword(KW::Delete)) => {
                let delete = self.spanned(Self::parse_delete)?;
                return Some(loc((start, delete.span.end), Statement::Delete(delete)));
            }
            _ => {}
        }
        let query = self.spanned(Self::parse_query)?;
        Some(loc((start, query.span.end), Statement::Query(query)))
    }

    pub fn parse_query(&mut self) -> R<Query> {
        Some(Query {
            with: self.try_spanned(Self::parse_with),
            body: self.try_spanned(Self::parse_query_expr),
            tail: self.try_spanned(Self::parse_query_suffix),
        })
    }

    fn parse_with(&mut self) -> R<With> {
        self.kw(KW::With)?;
        let recursive = self.kw(KW::Recursive).is_some();
        let mut ctes = Vec::new();
        loop {
            let start = self.pos();
            let name = self.ident()?;
            let columns = self.paren_list(|s| s.ident());
            let materialized = if self.kw(KW::Materialized).is_some() {
                Some(CteMaterialization::Materialized)
            } else if self.op(OpTag::Not).is_some() && self.kw(KW::Materialized).is_some() {
                Some(CteMaterialization::NotMaterialized)
            } else {
                None
            };
            self.kw(KW::As)?;
            let query = Box::new(self.parenthesized(|s| s.spanned(Self::parse_query))?);
            ctes.push(loc(
                self.span_from(start),
                Cte {
                    name,
                    columns,
                    materialized,
                    query,
                },
            ));
            if self.eat(TK::Comma).is_none() {
                break;
            }
        }
        Some(With {
            recursive,
            ctes: ctes.into(),
        })
    }

    fn parse_query_expr(&mut self) -> R<QueryExpr> {
        let left = self.spanned(Self::parse_query_primary)?;
        let set_ops = self.many(|s| s.spanned(Self::parse_set_op_term));
        Some(QueryExpr {
            left,
            set_ops: set_ops.into(),
        })
    }

    fn parse_set_op_term(&mut self) -> R<SetOpTerm> {
        let op = self.parse_set_op()?;
        Some(SetOpTerm {
            op,
            right: self.spanned(Self::parse_query_primary)?,
        })
    }

    fn parse_set_op(&mut self) -> R<SetOp> {
        let kw = match self.kind()? {
            TK::Keyword(kw @ (KW::Union | KW::Intersect | KW::Except | KW::Minus)) => kw,
            _ => return None,
        };
        self.advance();
        let all = self.kw(KW::All).is_some();
        let op = match kw {
            KW::Union => SetOp::Union { all },
            KW::Intersect => SetOp::Intersect { all },
            KW::Except => SetOp::Except { all },
            KW::Minus => SetOp::Minus { all },
            _ => unreachable!(),
        };
        Some(op)
    }

    fn parse_query_primary(&mut self) -> R<QueryPrimary> {
        match self.kind()? {
            TK::Keyword(KW::Select) => {
                Some(QueryPrimary::Select(self.spanned(Self::parse_select)?))
            }
            TK::Keyword(KW::Values) => {
                Some(QueryPrimary::Values(self.spanned(Self::parse_values)?))
            }
            _ => None,
        }
    }

    // ========================================================================
    // DML Statements
    // ========================================================================

    fn parse_insert(&mut self) -> R<Insert> {
        self.kw(KW::Insert)?;
        self.kw(KW::Into)?;
        let table = self.spanned(Self::parse_qualified_name)?;
        let columns = self.paren_list(|s| s.ident());

        let source = if self.kw(KW::Default).is_some() {
            self.kw(KW::Values);
            InsertSource::Default
        } else if self.kind() == Some(TK::Keyword(KW::Values)) {
            InsertSource::Values(self.spanned(Self::parse_values)?)
        } else {
            InsertSource::Query(Box::new(self.spanned(Self::parse_query)?))
        };

        let returning = self.parse_returning();
        Some(Insert {
            table,
            columns,
            source,
            returning,
        })
    }

    fn parse_values(&mut self) -> R<Values> {
        self.kw(KW::Values)?;
        let mut rows = Vec::new();
        loop {
            let row = self.parenthesized(|s| Some(s.args_list(Self::parse_expr)))?;
            rows.push(loc(self.span_from(self.pos()), row));
            if self.eat(TK::Comma).is_none() {
                break;
            }
        }
        Some(Values { rows })
    }

    fn parse_update(&mut self) -> R<Update> {
        self.kw(KW::Update)?;
        let table = self.spanned(Self::parse_qualified_name)?;
        let alias = self.alias();
        self.kw(KW::Set)?;
        let assignments = self.comma_list(Self::parse_assignment)?;
        let from = self.try_spanned(Self::parse_from);
        let where_clause = self.try_spanned(Self::parse_where);
        let returning = self.parse_returning();
        Some(Update {
            table,
            alias,
            assignments,
            from,
            where_clause,
            returning,
        })
    }

    fn parse_assignment(&mut self) -> R<Assignment> {
        let column = self.spanned(Self::ident)?;
        self.op(OpTag::Eq)?;
        let value = self.spanned(Self::parse_expr)?;
        Some(Assignment { column, value })
    }

    fn parse_delete(&mut self) -> R<Delete> {
        self.kw(KW::Delete)?;
        self.kw(KW::From)?;
        let table = self.spanned(Self::parse_qualified_name)?;
        let alias = self.alias();
        let using = if self.kw(KW::Using).is_some() {
            Some(loc(
                self.span_from(self.pos()),
                From {
                    sources: self.comma_list(|s| s.parse_table_ref()).unwrap_or_default(),
                },
            ))
        } else {
            None
        };
        let where_clause = self.try_spanned(Self::parse_where);
        let returning = self.parse_returning();
        Some(Delete {
            table,
            alias,
            using,
            where_clause,
            returning,
        })
    }

    fn parse_returning(&mut self) -> Option<Loc<Projection>> {
        self.kw(KW::Returning)?;
        self.spanned(Self::parse_projection)
    }

    fn parse_select(&mut self) -> R<Select> {
        self.kw(KW::Select)?;
        let distinct = self.parse_distinct().unwrap_or(SetQuantifier::All);
        let start = self.end(0);
        let proj = self.parse_projection().unwrap_or_default();
        Some(Select {
            distinct,
            projection: loc((start, self.end(start)), proj),
            from: self.try_spanned(Self::parse_from),
            where_clause: self.try_spanned(Self::parse_where),
            group_by: self.try_spanned(Self::parse_group_by),
            having: self.try_parse(Self::parse_having),
            window: self.try_spanned(Self::parse_window),
            qualify: None,
        })
    }

    fn parse_distinct(&mut self) -> R<SetQuantifier> {
        self.kw(KW::Distinct)?;
        if self.kw(KW::On).is_some() {
            let items = self.parenthesized(|s| Some(s.args_list(Self::parse_expr)))?;
            return Some(SetQuantifier::DistinctOn(items));
        }
        Some(SetQuantifier::Distinct)
    }

    fn parse_projection(&mut self) -> R<Projection> {
        Some(Projection {
            list: self
                .comma_list(|s| s.parse_projection_item())
                .unwrap_or_default(),
        })
    }

    fn parse_projection_item(&mut self) -> R<ProjectionItem> {
        Some(ProjectionItem {
            expr: self.spanned(|s| Some(s.parse_expr().unwrap_or(Expr::Empty)))?,
            alias: self.alias(),
        })
    }

    pub fn parse_expr(&mut self) -> R<Expr> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> R<Expr> {
        let start = self.pos();
        let mut lhs = self.spanned(Self::parse_prefix_expr)?;
        while let Some((tag, prec, r_bp)) = self.get_infix_op() {
            if prec < min_bp {
                break;
            }
            lhs = self.parse_infix(lhs, start, tag, r_bp)?;
        }
        Some(lhs.item)
    }

    fn parse_infix(&mut self, left: Loc<Expr>, start: usize, tag: OpTag, r_bp: u8) -> R<Loc<Expr>> {
        self.advance();
        match tag {
            OpTag::Not => self.parse_negated_predicate(left, start),
            OpTag::Between => self.parse_between(left, start, false),
            OpTag::Like | OpTag::Ilike => self.parse_like(left, start, tag, false),
            OpTag::Similar => self.parse_similar(left, start, false),
            OpTag::In => self.parse_in(left, start, false),
            OpTag::Is => self.parse_is(left, start),
            OpTag::TypeCast => self.parse_type_cast(left, start),
            _ => {
                let right = self.spanned(|s| s.parse_expr_bp(r_bp));
                let end = right
                    .as_ref()
                    .map(|r| r.span.end)
                    .unwrap_or_else(|| self.end(start));
                Some(loc(
                    (start, end),
                    Expr::Binary(loc(
                        (left.span.start, end),
                        Binary {
                            left: Box::new(left),
                            op: Some(loc(start, tag)),
                            right: right.map(Box::new),
                        },
                    )),
                ))
            }
        }
    }

    fn parse_negated_predicate(&mut self, left: Loc<Expr>, start: usize) -> R<Loc<Expr>> {
        // NOT followed by BETWEEN, LIKE, ILIKE, SIMILAR, IN
        match self.current()?.kind {
            TK::Operator(op) if op.semantic_tag == OpTag::Between => {
                self.advance();
                self.parse_between(left, start, true)
            }
            TK::Operator(op) if op.semantic_tag == OpTag::Like => {
                self.advance();
                self.parse_like(left, start, OpTag::Like, true)
            }
            TK::Operator(op) if op.semantic_tag == OpTag::Ilike => {
                self.advance();
                self.parse_like(left, start, OpTag::Ilike, true)
            }
            TK::Operator(op) if op.semantic_tag == OpTag::Similar => {
                self.advance();
                self.parse_similar(left, start, true)
            }
            TK::Operator(op) if op.semantic_tag == OpTag::In => {
                self.advance();
                self.parse_in(left, start, true)
            }
            _ => {
                // Just a regular NOT - parse as unary applied to rest of expression
                let expr = Box::new(self.spanned(|s| s.parse_expr_bp(3))?);
                Some(loc(
                    (start, expr.span.end),
                    Expr::Unary(loc(
                        (left.span.start, expr.span.end),
                        Unary {
                            op_tok: loc(start, OpTag::Not),
                            expr,
                        },
                    )),
                ))
            }
        }
    }

    fn parse_between(&mut self, left: Loc<Expr>, start: usize, not: bool) -> R<Loc<Expr>> {
        let low = self.spanned(|s| s.parse_expr_bp(10));
        if let Some(low) = low {
            if self.op(OpTag::And).is_some() {
                if let Some(high) = self.spanned(|s| s.parse_expr_bp(10)) {
                    return Some(loc(
                        (start, high.span.end),
                        Expr::Between(loc(
                            (start, high.span.end),
                            Between {
                                expr: Box::new(left),
                                low: Box::new(low),
                                high: Box::new(high),
                                not,
                            },
                        )),
                    ));
                }
            }
            return Some(loc(
                (start, low.span.end),
                Expr::Binary(loc(
                    left.span.join(low.span),
                    Binary {
                        left: Box::new(left),
                        op: Some(loc(start, OpTag::Between)),
                        right: Some(Box::new(low)),
                    },
                )),
            ));
        }
        // No low bound - partial BETWEEN
        let left_end = left.span.end;
        let left_start = left.span.start;
        let end = self.end(left_end);
        Some(loc(
            (start, end),
            Expr::Binary(loc(
                (left_start, end),
                Binary {
                    left: Box::new(left),
                    op: Some(loc(left_end, OpTag::Between)),
                    right: None,
                },
            )),
        ))
    }

    fn parse_like(&mut self, left: Loc<Expr>, start: usize, tag: OpTag, not: bool) -> R<Loc<Expr>> {
        let pattern = self.spanned(|s| s.parse_expr_bp(10))?;
        let span = (start, pattern.span.end);
        let expr = match tag {
            OpTag::Like => Expr::Like(loc(
                span,
                Like {
                    expr: Box::new(left),
                    pattern: Box::new(pattern),
                    not,
                },
            )),
            _ => Expr::ILike(loc(
                span,
                ILike {
                    expr: Box::new(left),
                    pattern: Box::new(pattern),
                    not,
                },
            )),
        };
        Some(loc(span, expr))
    }

    fn parse_similar(&mut self, left: Loc<Expr>, start: usize, _not: bool) -> R<Loc<Expr>> {
        self.kw(KW::To)?;
        let pattern = self.spanned(|s| s.parse_expr_bp(10))?;
        let escape = if self.kw(KW::Escape).is_some() {
            Some(Box::new(self.spanned(|s| s.parse_expr_bp(10))?))
        } else {
            None
        };
        let end = escape
            .as_ref()
            .map(|e| e.span.end)
            .unwrap_or(pattern.span.end);
        Some(loc(
            (start, end),
            Expr::Similar(loc(
                (start, end),
                Similar {
                    expr: Box::new(left),
                    pattern: Box::new(pattern),
                    escape,
                },
            )),
        ))
    }

    fn parse_in(&mut self, left: Loc<Expr>, start: usize, not: bool) -> R<Loc<Expr>> {
        self.eat(TK::LeftParen)?;
        let list = if matches!(self.kind(), Some(TK::Keyword(KW::Select | KW::With))) {
            ExprList::Subquery(Box::new(self.spanned(Self::parse_query)?))
        } else {
            ExprList::Exprs(self.args_list(Self::parse_expr).items)
        };
        self.eat(TK::RightParen);
        let span = self.span_from(start);
        Some(loc(
            span,
            Expr::In(loc(
                span,
                In {
                    expr: Box::new(left),
                    list,
                    not,
                },
            )),
        ))
    }

    fn parse_is(&mut self, left: Loc<Expr>, start: usize) -> R<Loc<Expr>> {
        let not = self.op(OpTag::Not).is_some();
        self.kw(KW::Null)?;
        let span = self.span_from(start);
        Some(loc(
            span,
            Expr::IsNull(loc(
                span,
                IsNull {
                    expr: Box::new(left),
                    not,
                },
            )),
        ))
    }

    fn parse_type_cast(&mut self, left: Loc<Expr>, start: usize) -> R<Loc<Expr>> {
        let data_type = self.spanned(Self::parse_data_type)?;
        let span = self.span_from(start);
        Some(loc(
            span,
            Expr::Cast(loc(
                span,
                Cast {
                    expr: Box::new(left),
                    data_type,
                },
            )),
        ))
    }

    fn parse_prefix_expr(&mut self) -> R<Expr> {
        if let Some(TK::Operator(op)) = self.kind() {
            // Explicit prefix operators (NOT, EXISTS)
            if op.fixity == Fixity::Prefix {
                if op.semantic_tag == OpTag::Exists {
                    self.advance();
                    let query = Box::new(self.parenthesized(|s| s.spanned(Self::parse_query))?);
                    return Some(Expr::Exists(query));
                }
                let tok = self.advance()?;
                let bp = op.precedence;
                let expr = Box::new(self.spanned(|s| s.parse_expr_bp(bp))?);
                return Some(Expr::Unary(loc(
                    tok.span.join(expr.span),
                    Unary {
                        op_tok: loc(tok.span, op.semantic_tag),
                        expr,
                    },
                )));
            }
            // Infix +/- used as prefix (unary plus/minus)
            if matches!(op.semantic_tag, OpTag::Add | OpTag::Sub) {
                let tok = self.advance()?;
                let tag = match op.semantic_tag {
                    OpTag::Add => OpTag::UnaryPlus,
                    _ => OpTag::UnaryMinus,
                };
                let bp = 8; // unary precedence
                let expr = Box::new(self.spanned(|s| s.parse_expr_bp(bp))?);
                return Some(Expr::Unary(loc(
                    tok.span.join(expr.span),
                    Unary {
                        op_tok: loc(tok.span, tag),
                        expr,
                    },
                )));
            }
        }
        self.parse_primary_expr()
    }

    fn parse_primary_expr(&mut self) -> R<Expr> {
        let start = self.pos();
        // Parenthesized or subquery
        if let Some(open) = self.eat(TK::LeftParen) {
            if matches!(self.kind(), Some(TK::Keyword(KW::Select | KW::With))) {
                let q = self.spanned(Self::parse_query)?;
                self.eat(TK::RightParen);
                return self.parse_postfix(start, Expr::Subquery(Box::new(q)));
            }
            let expr = Box::new(self.spanned(Self::parse_expr)?);
            let close = self.eat(TK::RightParen).map(|t| t.span);
            let paren = Expr::Paren(loc(
                (start, close.unwrap_or(expr.span).end),
                Paren {
                    open: open.span,
                    expr,
                    close,
                },
            ));
            return self.parse_postfix(start, paren);
        }
        // CAST expression
        if self.kw(KW::Cast).is_some() {
            return self
                .parse_cast(start)
                .and_then(|e| self.parse_postfix(start, e));
        }
        // CASE expression
        if self.kw(KW::Case).is_some() {
            return self
                .parse_case(start)
                .and_then(|e| self.parse_postfix(start, e));
        }
        // ROW(...)
        if self.kw(KW::Row).is_some() {
            let exprs = self.parenthesized(|s| Some(s.args_list(Self::parse_expr)))?;
            return self.parse_postfix(start, Expr::Row(loc(self.span_from(start), Row { exprs })));
        }
        // ARRAY[...]
        if self.kw(KW::Array).is_some() {
            self.eat(TK::LeftBracket)?;
            let items = self.args_list(Self::parse_expr);
            self.eat(TK::RightBracket);
            return self.parse_postfix(start, Expr::Array(items));
        }
        // Quantified: ANY/SOME/ALL(expr)
        if let Some(q) = self.parse_quantified(start) {
            return self.parse_postfix(start, q);
        }
        // Typed literals: DATE/TIME/TIMESTAMP 'string'
        if let Some(lit) = self.parse_typed_literal(start) {
            return self.parse_postfix(start, Expr::Literal(lit));
        }
        // Regular literals
        if let Some(lit) = self.parse_literal() {
            return self.parse_postfix(start, Expr::Literal(lit));
        }
        // Name or function call
        if let Some(name) = self.spanned(Self::parse_qualified_name) {
            if self.eat(TK::LeftParen).is_some() {
                return self.parse_func_call(name, start);
            }
            return self.parse_postfix(start, Expr::Name(name));
        }
        None
    }

    /// Parse postfix operations: subscripts [idx] or [lo:hi], AT TIME ZONE
    fn parse_postfix(&mut self, start: usize, mut expr: Expr) -> R<Expr> {
        loop {
            // Array subscript: expr[index] or expr[lo:hi]
            if self.eat(TK::LeftBracket).is_some() {
                let index = Box::new(self.spanned(Self::parse_expr)?);
                let upper = if self.eat(TK::Colon).is_some() {
                    Some(Box::new(self.spanned(Self::parse_expr)?))
                } else {
                    None
                };
                self.eat(TK::RightBracket);
                expr = Expr::Subscript(loc(
                    self.span_from(start),
                    Subscript {
                        expr: Box::new(loc(self.span_from(start), expr)),
                        index,
                        upper,
                    },
                ));
                continue;
            }
            // AT TIME ZONE 'tz'
            if self.kw(KW::At).is_some() {
                self.kw(KW::Time)?;
                self.kw(KW::Zone)?;
                let timezone = Box::new(self.spanned(Self::parse_expr)?);
                expr = Expr::AtTimeZone(loc(
                    self.span_from(start),
                    AtTimeZone {
                        expr: Box::new(loc(self.span_from(start), expr)),
                        timezone,
                    },
                ));
                continue;
            }
            break;
        }
        Some(expr)
    }

    fn parse_cast(&mut self, start: usize) -> R<Expr> {
        self.eat(TK::LeftParen)?;
        let expr = Box::new(self.spanned(Self::parse_expr)?);
        self.kw(KW::As)?;
        let data_type = self.spanned(Self::parse_data_type)?;
        self.eat(TK::RightParen);
        Some(Expr::Cast(loc(
            self.span_from(start),
            Cast { expr, data_type },
        )))
    }

    fn parse_data_type(&mut self) -> R<DataType> {
        let name = self.spanned(Self::parse_qualified_name)?;
        if self.eat(TK::LeftParen).is_some() {
            let mut params = Vec::new();
            loop {
                let param = match self.kind()? {
                    TK::Number => {
                        let tok = self.current()?;
                        let n = tok.text.parse().ok()?;
                        let span = tok.span;
                        self.advance();
                        loc(span, TypeParam::Number(n))
                    }
                    TK::Identifier | TK::IdentifierQuoted(_) => {
                        let span = self.current()?.span;
                        self.advance();
                        loc(span, TypeParam::Ident(span))
                    }
                    _ => break,
                };
                params.push(param);
                if self.eat(TK::Comma).is_none() {
                    break;
                }
            }
            self.eat(TK::RightParen);
            Some(DataType::Parameterized {
                name,
                params: params.into(),
            })
        } else {
            Some(DataType::Named(name))
        }
    }

    fn parse_case(&mut self, start: usize) -> R<Expr> {
        let operand = if self.kind() != Some(TK::Keyword(KW::When)) {
            Some(Box::new(self.spanned(Self::parse_expr)?))
        } else {
            None
        };
        let mut when_clauses = Vec::new();
        while self.kw(KW::When).is_some() {
            let when = self.spanned(Self::parse_expr)?;
            self.kw(KW::Then)?;
            let then = self.spanned(Self::parse_expr)?;
            when_clauses.push(WhenClause { when, then });
        }
        let else_clause = if self.kw(KW::Else).is_some() {
            Some(Box::new(self.spanned(Self::parse_expr)?))
        } else {
            None
        };
        self.kw(KW::End)?;
        Some(Expr::Case(loc(
            self.span_from(start),
            Case {
                operand,
                when_clauses,
                else_clause,
            },
        )))
    }

    fn parse_quantified(&mut self, start: usize) -> R<Expr> {
        let quantifier = match self.kind()? {
            TK::Keyword(KW::Any) => Quantifier::Any,
            TK::Keyword(KW::Some) => Quantifier::Some,
            TK::Keyword(KW::All) if self.peek_kind() == Some(TK::LeftParen) => Quantifier::All,
            _ => return None,
        };
        self.advance();
        self.eat(TK::LeftParen)?;
        // Check if it's a subquery
        let expr = if matches!(self.kind(), Some(TK::Keyword(KW::Select | KW::With))) {
            Box::new(
                self.spanned(|s| Some(Expr::Subquery(Box::new(s.spanned(Self::parse_query)?))))?,
            )
        } else {
            Box::new(self.spanned(Self::parse_expr)?)
        };
        self.eat(TK::RightParen);
        Some(Expr::Quantified(loc(
            self.span_from(start),
            Quantified { quantifier, expr },
        )))
    }

    fn parse_typed_literal(&mut self, start: usize) -> R<Loc<Literal>> {
        let data_type = match self.kind()? {
            TK::Keyword(KW::Date) => TypedLiteralKind::Date,
            TK::Keyword(KW::Time) => TypedLiteralKind::Time,
            TK::Keyword(KW::Timestamp) => TypedLiteralKind::Timestamp,
            TK::Keyword(KW::Interval) => TypedLiteralKind::Interval,
            _ => return None,
        };
        self.advance();
        let tok = self.eat(TK::Str)?;
        Some(loc(
            self.span_from(start),
            Literal::TypedString {
                data_type,
                value: tok.span,
            },
        ))
    }

    fn parse_func_call(&mut self, name: Loc<QualifiedName>, start: usize) -> R<Expr> {
        let distinct = self.kw(KW::Distinct).is_some();
        let args_start = self.pos();
        let args = self.args_list(Self::parse_expr);
        self.eat(TK::RightParen);
        let args_span = Span::new(args_start, self.end(args_start));
        let filter = if self.kw(KW::Filter).is_some() {
            Some(Box::new(self.parenthesized(|s| {
                s.kw(KW::Where);
                s.spanned(Self::parse_expr)
            })?))
        } else {
            None
        };
        let span = Span::new(name.span.start, self.end(name.span.end));
        let expr = if self.kw(KW::Over).is_some() {
            let over = self.parse_window_ref()?;
            Expr::Over(loc(
                span,
                Over {
                    name,
                    args: loc(args_span, args),
                    over,
                    filter,
                },
            ))
        } else {
            Expr::FunctionCall(loc(
                span,
                FunctionCall {
                    name,
                    distinct,
                    args: loc(args_span, args),
                    filter,
                },
            ))
        };
        self.parse_postfix(start, expr)
    }

    fn parse_literal(&mut self) -> R<Loc<Literal>> {
        let tok = self.current()?;
        let span = tok.span;
        let lit = match tok.kind {
            TK::Number => {
                self.advance();
                Literal::Number(tok.text.parse().ok()?)
            }
            TK::Float => {
                self.advance();
                Literal::Float(tok.text.parse().ok()?)
            }
            TK::Str => {
                self.advance();
                Literal::String(span)
            }
            TK::Keyword(KW::Null) => {
                self.advance();
                Literal::Null
            }
            TK::Keyword(KW::True) => {
                self.advance();
                Literal::Boolean(Boolean::True)
            }
            TK::Keyword(KW::False) => {
                self.advance();
                Literal::Boolean(Boolean::False)
            }
            TK::Keyword(KW::Unknown) => {
                self.advance();
                Literal::Boolean(Boolean::Unknown)
            }
            _ => return None,
        };
        Some(loc(span, lit))
    }

    fn get_infix_op(&self) -> Option<(OpTag, u8, u8)> {
        match self.current()?.kind {
            TK::Operator(op) if op.fixity == Fixity::Infix => {
                let r_bp = match op.assoc {
                    Assoc::Right => op.precedence,
                    Assoc::Left | Assoc::None => op.precedence + 1,
                };
                Some((op.semantic_tag, op.precedence, r_bp))
            }
            // NOT can be infix when followed by BETWEEN, LIKE, ILIKE, SIMILAR, IN
            TK::Operator(op) if op.semantic_tag == OpTag::Not => {
                let next = self.tokens.get(self.pos + 1)?;
                if let TK::Operator(next_op) = next.kind {
                    if matches!(
                        next_op.semantic_tag,
                        OpTag::Between | OpTag::Like | OpTag::Ilike | OpTag::Similar | OpTag::In
                    ) {
                        return Some((OpTag::Not, next_op.precedence, next_op.precedence + 1));
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn parse_from(&mut self) -> R<From> {
        self.kw(KW::From)?;
        Some(From {
            sources: self.comma_list(|s| s.parse_table_ref()).unwrap_or_default(),
        })
    }

    fn parse_table_ref(&mut self) -> R<TableRef> {
        let start = self.pos();
        let mut left = self.parse_table_factor()?;
        while let Some(kind) = self.parse_join_kind() {
            let rs = self.pos();
            let right = self.parse_table_factor()?;
            let constraint = self.try_spanned(Self::parse_join_constraint);
            let span = Span::new(start, self.end(rs));
            let right_span = Span::new(rs, span.end);
            left = TableRef::Join(loc(
                span,
                Join {
                    left: Box::new(loc(span, left)),
                    kind,
                    right: Box::new(loc(right_span, right)),
                    constraint,
                },
            ));
        }
        Some(left)
    }

    fn parse_table_factor(&mut self) -> R<TableRef> {
        let start = self.pos();
        let lateral = self.kw(KW::Lateral).is_some();
        if self.eat(TK::LeftParen).is_some() {
            if matches!(self.kind(), Some(TK::Keyword(KW::Select | KW::With))) {
                let query = self.spanned(Self::parse_query)?;
                self.eat(TK::RightParen);
                let alias = self.alias();
                let span = self.span_from(start);
                return Some(TableRef::Factor(loc(
                    span,
                    TableFactor::Subquery(loc(
                        span,
                        SubqueryTableFactor {
                            query,
                            alias,
                            lateral,
                        },
                    )),
                )));
            }
            let inner = Box::new(self.spanned(Self::parse_table_ref)?);
            self.eat(TK::RightParen);
            return Some(TableRef::Factor(loc(
                self.span_from(start),
                TableFactor::Parenthesized(inner),
            )));
        }
        let name = self.spanned(Self::parse_qualified_name)?;
        if self.eat(TK::LeftParen).is_some() {
            let args = self.args_list(Self::parse_expr);
            self.eat(TK::RightParen);
            let alias = self.alias();
            // Column aliases can include keywords like "key", "value"
            let columns = self.paren_list(|s| s.ident_or_keyword());
            let span = self.span_from(start);
            return Some(TableRef::Factor(loc(
                span,
                TableFactor::Function(loc(
                    span,
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
        let alias = self.alias();
        let span = self.span_from(start);
        Some(TableRef::Factor(loc(
            span,
            TableFactor::Named(loc(
                span,
                NamedTableFactor {
                    name,
                    alias,
                    lateral,
                },
            )),
        )))
    }

    fn parse_join_kind(&mut self) -> R<JoinKind> {
        const OUTER_JOINS: &[(KW, JoinBase)] = &[
            (KW::Left, JoinBase::Left),
            (KW::Right, JoinBase::Right),
            (KW::Full, JoinBase::Full),
        ];

        let natural = self.kw(KW::Natural).is_some();
        let (base, outer) = match self.kind()? {
            TK::Keyword(KW::Inner) => {
                self.advance();
                self.kw(KW::Join)?;
                (JoinBase::Inner, false)
            }
            TK::Keyword(kw) if OUTER_JOINS.iter().any(|(k, _)| *k == kw) => {
                self.advance();
                let outer = self.kw(KW::Outer).is_some();
                self.kw(KW::Join)?;
                let base = OUTER_JOINS.iter().find(|(k, _)| *k == kw).unwrap().1;
                (base, outer)
            }
            TK::Keyword(KW::Cross) => {
                self.advance();
                self.kw(KW::Join)?;
                (JoinBase::Cross, false)
            }
            TK::Keyword(KW::Join) => {
                self.advance();
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

    fn parse_join_constraint(&mut self) -> R<JoinConstraint> {
        if self.kw(KW::On).is_some() {
            return Some(JoinConstraint::On(self.spanned(Self::parse_expr)?));
        }
        if self.kw(KW::Using).is_some() {
            let cols = self.parenthesized(|s| s.comma_list(Self::ident))?;
            return Some(JoinConstraint::Using(cols));
        }
        None
    }

    fn parse_where(&mut self) -> R<Where> {
        self.kw(KW::Where)?;
        Some(Where {
            expr: self.spanned(|s| Some(s.parse_expr().unwrap_or(Expr::Empty)))?,
        })
    }

    fn parse_group_by(&mut self) -> R<GroupBy> {
        self.kw_seq(&[KW::Group, KW::By])?;
        Some(GroupBy {
            items: self
                .comma_list(Self::parse_group_by_item)
                .unwrap_or_default(),
        })
    }

    fn parse_group_by_item(&mut self) -> R<GroupByItem> {
        // ROLLUP(a, b, ...)
        if self.kw(KW::Rollup).is_some() {
            let exprs = self.parenthesized(|s| s.comma_list(Self::parse_expr))?;
            return Some(GroupByItem::Rollup(exprs.items.into()));
        }
        // CUBE(a, b, ...)
        if self.kw(KW::Cube).is_some() {
            let exprs = self.parenthesized(|s| s.comma_list(Self::parse_expr))?;
            return Some(GroupByItem::Cube(exprs.items.into()));
        }
        // GROUPING SETS((a, b), (a), ...)
        if self.kw(KW::Grouping).is_some() && self.kw(KW::Sets).is_some() {
            let sets = self.parenthesized(|s| s.comma_list(Self::parse_grouping_set))?;
            return Some(GroupByItem::GroupingSets(sets.items.into()));
        }
        // Simple expression
        Some(GroupByItem::Expr(self.spanned(Self::parse_expr)?))
    }

    fn parse_grouping_set(&mut self) -> R<GroupingSet> {
        // Empty set: ()
        if self
            .try_parse(|s| {
                s.eat(TK::LeftParen)?;
                s.eat(TK::RightParen)
            })
            .is_some()
        {
            return Some(GroupingSet::Exprs(Vec::new().into()));
        }
        // Parenthesized list: (a, b)
        if let Some(exprs) = self.try_parse(|s| s.parenthesized(|s| s.comma_list(Self::parse_expr)))
        {
            return Some(GroupingSet::Exprs(exprs.items.into()));
        }
        // Single expression
        Some(GroupingSet::Expr(self.spanned(Self::parse_expr)?))
    }

    fn parse_having(&mut self) -> R<Loc<Expr>> {
        self.kw(KW::Having)?;
        self.spanned(|s| Some(s.parse_expr().unwrap_or(Expr::Empty)))
    }

    fn parse_window(&mut self) -> R<Window> {
        self.kw(KW::Window)?;
        let mut windows = Vec::new();
        loop {
            let name = self.ident()?;
            self.kw(KW::As)?;
            let spec = self.parenthesized(|s| s.spanned(Self::parse_window_spec))?;
            windows.push(loc(name, WindowDef { name, spec }));
            if self.eat(TK::Comma).is_none() {
                break;
            }
        }
        Some(Window {
            windows: windows.into(),
        })
    }

    fn parse_window_ref(&mut self) -> R<WindowRef> {
        if let Some(spec) =
            self.try_parse(|s| s.parenthesized(|s| s.spanned(Self::parse_window_spec)))
        {
            return Some(WindowRef::Spec(Box::new(spec)));
        }
        Some(WindowRef::Name(self.spanned(Self::ident)?))
    }

    fn parse_window_spec(&mut self) -> R<WindowSpec> {
        let partition_by = self
            .kw_seq(&[KW::Partition, KW::By])
            .and_then(|_| self.comma_list(Self::parse_expr));
        Some(WindowSpec {
            partition_by,
            order_by: self.try_spanned(Self::parse_order_by),
            frame: self.try_spanned(Self::parse_window_frame).map(Box::new),
        })
    }

    fn parse_window_frame(&mut self) -> R<WindowFrame> {
        let unit = match self.kind()? {
            TK::Keyword(KW::Rows) => {
                self.advance();
                FrameUnit::Rows
            }
            TK::Keyword(KW::Range) => {
                self.advance();
                FrameUnit::Range
            }
            _ => return None,
        };
        self.op(OpTag::Between)?;
        let start = Box::new(self.spanned(Self::parse_frame_bound)?);
        self.op(OpTag::And);
        let end = Some(Box::new(self.spanned(Self::parse_frame_bound)?));
        Some(WindowFrame { unit, start, end })
    }

    fn parse_frame_bound(&mut self) -> R<FrameBound> {
        match self.kind()? {
            TK::Keyword(KW::Unbounded) => {
                self.advance();
                if self.kw(KW::Preceding).is_some() {
                    Some(FrameBound::UnboundedPreceding)
                } else {
                    self.kw(KW::Following)?;
                    Some(FrameBound::UnboundedFollowing)
                }
            }
            TK::Keyword(KW::Current) => {
                self.advance();
                self.kw(KW::Row)?;
                Some(FrameBound::CurrentRow)
            }
            _ => {
                let expr = Box::new(self.spanned(Self::parse_expr)?);
                if self.kw(KW::Preceding).is_some() {
                    Some(FrameBound::Preceding(expr))
                } else {
                    self.kw(KW::Following)?;
                    Some(FrameBound::Following(expr))
                }
            }
        }
    }

    fn parse_query_suffix(&mut self) -> R<QuerySuffix> {
        let (mut order_by, mut limit, mut offset) = (None, None, None);
        loop {
            let parsed = order_by.is_none() && {
                order_by = self.try_spanned(Self::parse_order_by);
                order_by.is_some()
            } || limit.is_none() && {
                limit = self.try_spanned(Self::parse_limit);
                limit.is_some()
            } || offset.is_none() && {
                offset = self.try_spanned(Self::parse_offset);
                offset.is_some()
            };
            if !parsed {
                break;
            }
        }
        (order_by.is_some() || limit.is_some() || offset.is_some()).then_some(QuerySuffix {
            order_by,
            limit,
            offset,
        })
    }

    fn parse_order_by(&mut self) -> R<OrderBy> {
        self.kw_seq(&[KW::Order, KW::By])?;
        Some(OrderBy {
            items: self.comma_list(Self::parse_order_by_item)?,
        })
    }

    fn parse_order_by_item(&mut self) -> R<OrderByItem> {
        let expr = self.spanned(Self::parse_expr)?;
        let direction = match self.kind() {
            Some(TK::Keyword(KW::Asc)) => {
                self.advance();
                Some(SortDirection::Asc)
            }
            Some(TK::Keyword(KW::Desc)) => {
                self.advance();
                Some(SortDirection::Desc)
            }
            _ => None,
        };
        let nulls = if self.kw(KW::Nulls).is_some() {
            match self.kind() {
                Some(TK::Keyword(KW::First)) => {
                    self.advance();
                    Some(NullsOrder::First)
                }
                Some(TK::Keyword(KW::Last)) => {
                    self.advance();
                    Some(NullsOrder::Last)
                }
                _ => None,
            }
        } else {
            None
        };
        Some(OrderByItem {
            expr,
            direction,
            nulls,
        })
    }

    fn parse_limit(&mut self) -> R<Limit> {
        if self.kw_seq(&[KW::Fetch, KW::First]).is_some() {
            let count = self.spanned(Self::parse_expr)?;
            self.kw(KW::Rows);
            self.kw(KW::Only);
            return Some(Limit {
                count,
                style: LimitKind::FetchFirst,
            });
        }
        self.kw(KW::Limit)?;
        Some(Limit {
            count: self.spanned(Self::parse_expr)?,
            style: LimitKind::Limit,
        })
    }

    fn parse_offset(&mut self) -> R<Offset> {
        self.kw(KW::Offset)?;
        Some(Offset {
            count: self.spanned(Self::parse_expr)?,
            rows_keyword: self.kw(KW::Rows).is_some(),
        })
    }

    fn parse_qualified_name(&mut self) -> R<QualifiedName> {
        Some(QualifiedName {
            parts: self.delimited_list(TK::Dot, |s| s.parse_name_part())?,
        })
    }

    fn parse_name_part(&mut self) -> R<NamePart> {
        if let Some(id) = self.ident() {
            return Some(NamePart::Ident(id));
        }
        if self.op(OpTag::Mul).is_some() {
            return Some(NamePart::Star);
        }
        if let Some(TK::Keyword(kw)) = self.kind() {
            if !is_reserved(kw) {
                let sp = self.current()?.span;
                self.advance();
                return Some(NamePart::Ident(sp));
            }
        }
        None
    }

    fn ident(&mut self) -> R<Identifier> {
        match self.kind() {
            Some(TK::Identifier | TK::IdentifierQuoted(_)) => {
                let sp = self.current()?.span;
                self.advance();
                Some(sp)
            }
            _ => None,
        }
    }

    fn ident_or_keyword(&mut self) -> R<Identifier> {
        match self.kind() {
            Some(TK::Identifier | TK::IdentifierQuoted(_) | TK::Keyword(_)) => {
                let sp = self.current()?.span;
                self.advance();
                Some(sp)
            }
            _ => None,
        }
    }

    fn alias(&mut self) -> R<Loc<Identifier>> {
        if self.kw(KW::As).is_some() {
            return self.spanned(Self::ident);
        }
        match self.kind() {
            Some(TK::Identifier | TK::IdentifierQuoted(_)) => self.spanned(Self::ident),
            _ => None,
        }
    }

    // --- Utilities ---
    #[inline]
    fn eat(&mut self, k: TK) -> R<&'tok Token<'txt>> {
        if self.kind() == Some(k) {
            self.advance()
        } else {
            None
        }
    }

    #[inline]
    fn kw(&mut self, k: KW) -> R<&'tok Token<'txt>> {
        self.eat(TK::Keyword(k))
    }

    #[inline]
    fn op(&mut self, t: OpTag) -> R<&'tok Token<'txt>> {
        match self.kind() {
            Some(TK::Operator(o)) if o.semantic_tag == t => self.advance(),
            _ => None,
        }
    }

    #[inline]
    fn current(&self) -> Option<&'tok Token<'txt>> {
        self.tokens.get(self.pos)
    }

    #[inline]
    fn kind(&self) -> Option<TK> {
        self.current().map(|t| t.kind)
    }

    #[inline]
    fn advance(&mut self) -> Option<&'tok Token<'txt>> {
        let t = self.tokens.get(self.pos)?;
        self.pos += 1;
        Some(t)
    }

    #[inline]
    fn pos(&self) -> usize {
        self.current()
            .map(|t| t.span.start)
            .unwrap_or_else(|| self.prev_tok().map(|t| t.span.end).unwrap_or(0))
    }

    #[inline]
    fn end(&self, fallback: usize) -> usize {
        self.prev_tok().map(|t| t.span.end).unwrap_or(fallback)
    }

    #[inline]
    fn span(&self) -> Span {
        self.current().map(|t| t.span).unwrap_or(Span::new(0, 0))
    }

    #[inline]
    fn peek_kind(&self) -> Option<TK> {
        self.tokens.get(self.pos + 1).map(|t| t.kind)
    }

    #[inline]
    fn prev_tok(&self) -> Option<&'tok Token<'txt>> {
        self.pos.checked_sub(1).and_then(|i| self.tokens.get(i))
    }

    #[inline]
    fn is_eof(&self) -> bool {
        matches!(self.kind(), None | Some(TK::Eof))
    }

    fn spanned<T>(&mut self, f: impl FnOnce(&mut Self) -> R<T>) -> R<Loc<T>> {
        let start = self.pos();
        let v = f(self)?;
        Some(loc((start, self.end(start)), v))
    }

    fn try_parse<T>(&mut self, f: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
        let cp = self.pos;
        f(self).or_else(|| {
            self.pos = cp;
            None
        })
    }

    fn try_spanned<T>(&mut self, f: impl FnOnce(&mut Self) -> R<T>) -> R<Loc<T>> {
        self.try_parse(|s| s.spanned(f))
    }

    fn many<T>(&mut self, mut f: impl FnMut(&mut Self) -> R<T>) -> Vec<T> {
        let mut v = Vec::new();
        while let Some(x) = self.try_parse(&mut f) {
            v.push(x);
        }
        v
    }

    fn delimited_list<T>(
        &mut self, sep: TK, mut f: impl FnMut(&mut Self) -> R<T>,
    ) -> R<DelimitedList<Loc<T>>> {
        let mut items = vec![self.spanned(&mut f)?];
        let mut seps = Vec::new();
        loop {
            let cp = self.pos;
            if let Some(t) = self.eat(sep) {
                seps.push(loc(t.span, sep));
                match self.spanned(&mut f) {
                    Some(x) => items.push(x),
                    None => break,
                }
            } else {
                self.pos = cp;
                break;
            }
        }
        Some(DelimitedList { items, seps })
    }

    fn comma_list<T>(&mut self, f: impl FnMut(&mut Self) -> R<T>) -> R<DelimitedList<Loc<T>>> {
        self.delimited_list(TK::Comma, f)
    }

    fn paren_list<T>(&mut self, f: impl FnMut(&mut Self) -> R<T>) -> Option<DelimitedList<Loc<T>>> {
        self.eat(TK::LeftParen)?;
        let list = self.args_list(f);
        self.eat(TK::RightParen);
        Some(list)
    }

    fn parenthesized<T>(&mut self, f: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
        self.eat(TK::LeftParen)?;
        let v = f(self);
        self.eat(TK::RightParen);
        v
    }

    fn args_list<T>(&mut self, f: impl FnMut(&mut Self) -> R<T>) -> DelimitedList<Loc<T>> {
        if self.kind() == Some(TK::RightParen) {
            DelimitedList::default()
        } else {
            self.comma_list(f).unwrap_or_default()
        }
    }

    fn span_from(&self, start: usize) -> Span {
        Span::new(start, self.end(start))
    }

    fn kw_seq(&mut self, kws: &[KW]) -> R<()> {
        for &k in kws {
            self.kw(k)?;
        }
        Some(())
    }
}

impl<'txt, 'tok> Iterator for WinnowParser<'txt, 'tok> {
    type Item = Loc<Statement>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_eof() {
            None
        } else {
            self.parse_statement()
        }
    }
}

#[inline]
fn loc<T>(span: impl Into<Span>, v: T) -> Loc<T> {
    Loc::new(span, v)
}

fn is_reserved(kw: KW) -> bool {
    matches!(
        kw,
        KW::Select
            | KW::From
            | KW::Where
            | KW::Group
            | KW::Having
            | KW::Order
            | KW::Limit
            | KW::Offset
            | KW::Join
            | KW::Inner
            | KW::Left
            | KW::Right
            | KW::Full
            | KW::Cross
            | KW::On
            | KW::Using
            | KW::Union
            | KW::Intersect
            | KW::Except
            | KW::With
            | KW::As
            | KW::By
            | KW::Distinct
            | KW::All
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::DialectKind;
    use crate::lex;

    fn parse(sql: &str) -> R<Loc<Statement>> {
        WinnowParser::new(
            &lex::lex(DialectKind::Postgres.spec(), sql),
            DialectKind::Postgres.spec(),
        )
        .parse_statement()
    }

    #[test]
    fn simple_select() {
        assert!(parse("SELECT 1").is_some());
    }
    #[test]
    fn select_columns() {
        assert!(parse("SELECT a, b, c FROM t").is_some());
    }
    #[test]
    fn with_clause() {
        assert!(matches!(
            parse("WITH cte AS (SELECT 1) SELECT * FROM cte"),
            Some(Loc { item: Statement::Query(q), .. }) if q.with.is_some()
        ));
    }
    #[test]
    fn join() {
        assert!(parse("SELECT * FROM a JOIN b ON a.id = b.id").is_some());
    }
    #[test]
    fn union() {
        assert!(matches!(
            parse("SELECT 1 UNION ALL SELECT 2"),
            Some(Loc { item: Statement::Query(q), .. }) if q.body.as_ref().map(|b| !b.set_ops.is_empty()).unwrap_or(false)
        ));
    }

    #[test]
    fn cast_expr() {
        assert!(parse("SELECT CAST(x AS INTEGER)").is_some());
        assert!(parse("SELECT CAST(y AS VARCHAR(255))").is_some());
        assert!(parse("SELECT CAST(z AS NUMERIC(10, 2))").is_some());
    }

    #[test]
    fn interval_literal() {
        assert!(parse("SELECT INTERVAL '1 day'").is_some());
    }

    #[test]
    fn nulls_order() {
        assert!(parse("SELECT * FROM t ORDER BY x NULLS FIRST").is_some());
        assert!(parse("SELECT * FROM t ORDER BY x DESC NULLS LAST").is_some());
    }

    #[test]
    fn not_predicates() {
        assert!(parse("SELECT * FROM t WHERE x NOT BETWEEN 1 AND 10").is_some());
        assert!(parse("SELECT * FROM t WHERE x NOT LIKE '%foo%'").is_some());
        assert!(parse("SELECT * FROM t WHERE x NOT IN (1, 2, 3)").is_some());
    }

    #[test]
    fn postgres_type_cast() {
        assert!(parse("SELECT x::integer").is_some());
        assert!(parse("SELECT x::text::integer").is_some());
        assert!(parse("SELECT (a + b)::numeric(10,2)").is_some());
    }

    #[test]
    fn array_subscript() {
        assert!(parse("SELECT arr[1]").is_some());
        assert!(parse("SELECT arr[1:3]").is_some());
        assert!(parse("SELECT matrix[1][2]").is_some());
    }

    #[test]
    fn row_constructor() {
        assert!(parse("SELECT ROW(1, 2, 3)").is_some());
        assert!(parse("SELECT ROW(a, b, c) FROM t").is_some());
    }

    #[test]
    fn at_time_zone() {
        assert!(parse("SELECT ts AT TIME ZONE 'UTC'").is_some());
        assert!(parse("SELECT created_at AT TIME ZONE 'America/New_York' FROM t").is_some());
    }

    #[test]
    fn overlaps() {
        assert!(parse("SELECT (a, b) OVERLAPS (c, d)").is_some());
    }

    // DML tests
    #[test]
    fn insert_values() {
        assert!(matches!(
            parse("INSERT INTO t (a, b) VALUES (1, 2)"),
            Some(Loc {
                item: Statement::Insert(_),
                ..
            })
        ));
    }

    #[test]
    fn insert_multiple_rows() {
        assert!(matches!(
            parse("INSERT INTO t VALUES (1), (2), (3)"),
            Some(Loc {
                item: Statement::Insert(_),
                ..
            })
        ));
    }

    #[test]
    fn insert_from_select() {
        assert!(matches!(
            parse("INSERT INTO t SELECT * FROM s"),
            Some(Loc {
                item: Statement::Insert(_),
                ..
            })
        ));
    }

    #[test]
    fn insert_default() {
        assert!(matches!(
            parse("INSERT INTO t DEFAULT VALUES"),
            Some(Loc {
                item: Statement::Insert(_),
                ..
            })
        ));
    }

    #[test]
    fn insert_returning() {
        assert!(matches!(
            parse("INSERT INTO t (a) VALUES (1) RETURNING *"),
            Some(Loc {
                item: Statement::Insert(_),
                ..
            })
        ));
    }

    #[test]
    fn update_simple() {
        assert!(matches!(
            parse("UPDATE t SET a = 1 WHERE id = 1"),
            Some(Loc {
                item: Statement::Update(_),
                ..
            })
        ));
    }

    #[test]
    fn update_multiple_cols() {
        assert!(matches!(
            parse("UPDATE t SET a = 1, b = 2, c = 3"),
            Some(Loc {
                item: Statement::Update(_),
                ..
            })
        ));
    }

    #[test]
    fn update_from() {
        assert!(matches!(
            parse("UPDATE t SET a = s.a FROM s WHERE t.id = s.id"),
            Some(Loc {
                item: Statement::Update(_),
                ..
            })
        ));
    }

    #[test]
    fn update_returning() {
        assert!(matches!(
            parse("UPDATE t SET a = 1 RETURNING *"),
            Some(Loc {
                item: Statement::Update(_),
                ..
            })
        ));
    }

    #[test]
    fn delete_simple() {
        assert!(matches!(
            parse("DELETE FROM t WHERE id = 1"),
            Some(Loc {
                item: Statement::Delete(_),
                ..
            })
        ));
    }

    #[test]
    fn delete_using() {
        assert!(matches!(
            parse("DELETE FROM t USING s WHERE t.id = s.id"),
            Some(Loc {
                item: Statement::Delete(_),
                ..
            })
        ));
    }

    #[test]
    fn delete_returning() {
        assert!(matches!(
            parse("DELETE FROM t WHERE id = 1 RETURNING *"),
            Some(Loc {
                item: Statement::Delete(_),
                ..
            })
        ));
    }

    #[test]
    fn values_standalone() {
        assert!(parse("VALUES (1, 2), (3, 4)").is_some());
    }

    #[test]
    fn values_in_cte() {
        assert!(parse("WITH v AS (VALUES (1), (2)) SELECT * FROM v").is_some());
    }

    #[test]
    fn group_by_rollup() {
        let stmt = parse("SELECT a, b, SUM(c) FROM t GROUP BY ROLLUP(a, b)");
        assert!(stmt.is_some());
    }

    #[test]
    fn group_by_cube() {
        let stmt = parse("SELECT a, b, SUM(c) FROM t GROUP BY CUBE(a, b)");
        assert!(stmt.is_some());
    }

    #[test]
    fn group_by_grouping_sets() {
        let stmt = parse("SELECT a, b, SUM(c) FROM t GROUP BY GROUPING SETS((a, b), (a), (b), ())");
        assert!(stmt.is_some());
    }

    #[test]
    fn group_by_mixed() {
        let stmt = parse("SELECT a, b, c FROM t GROUP BY a, ROLLUP(b, c)");
        assert!(stmt.is_some());
    }
}
