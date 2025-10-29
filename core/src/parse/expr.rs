use crate::ast::*;
use crate::lex::Assoc;
use crate::lex::Keyword;
use crate::lex::OpTag;
use crate::lex::Operator;
use crate::lex::TokenKind;
use crate::parse::Parser;
use crate::span::Loc;

/// Pratt parser for expressions.
impl<'txt, 'tok> Parser<'txt, 'tok> {
    pub(crate) fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_expr_prec(0)
    }

    fn parse_expr_prec(&mut self, min_prec: u8) -> Option<Expr> {
        let start_pos = self.tokens.current().map(|t| t.span.start).unwrap_or(0);
        let mut left = self.parse_prefix()?;
        while let Some(prec) = self.infix_binding_power() {
            if prec < min_prec {
                break;
            }
            left = self.parse_infix(left, start_pos, prec)?;
        }

        Some(left)
    }

    fn parse_prefix(&mut self) -> Option<Expr> {
        let span = self.tokens.current().map(|t| t.span).unwrap_or(0.into());

        match self.tokens.current_kind()? {
            // Literals
            TokenKind::Number => {
                let tok = self.tokens.advance()?;
                let val = tok.text.parse::<i64>().ok()?;
                Some(Expr::Literal(Loc::new(span, Literal::Number(val))))
            }
            TokenKind::Float => {
                let tok = self.tokens.advance()?;
                let val = tok.text.parse::<f64>().ok()?;
                Some(Expr::Literal(Loc::new(span, Literal::Float(val))))
            }
            TokenKind::Str => {
                let tok = self.tokens.advance()?;
                Some(Expr::Literal(Loc::new(tok.span, Literal::String(tok.span))))
            }
            TokenKind::Keyword(Keyword::Null) => {
                self.tokens.advance()?;
                Some(Expr::Literal(Loc::new(span, Literal::Null)))
            }
            TokenKind::Keyword(kw @ (Keyword::True | Keyword::False | Keyword::Unknown)) => {
                self.tokens.advance()?;
                let b = match kw {
                    Keyword::True => Boolean::True,
                    Keyword::False => Boolean::False,
                    Keyword::Unknown => Boolean::Unknown,
                    _ => unreachable!(),
                };
                Some(Expr::Literal(Loc::new(span, Literal::Boolean(b))))
            }

            // Typed literals: DATE 'string', TIME 'string', TIMESTAMP 'string'
            TokenKind::Keyword(kw @ (Keyword::Date | Keyword::Time | Keyword::Timestamp)) => {
                let start = self.tokens.current().map(|t| t.span.start).unwrap_or(0);
                let data_type = match kw {
                    Keyword::Date => TypedLiteralKind::Date,
                    Keyword::Time => TypedLiteralKind::Time,
                    Keyword::Timestamp => TypedLiteralKind::Timestamp,
                    _ => unreachable!(),
                };
                self.tokens.advance()?;
                let end = self.tokens.prev().map(|t| t.span.end).unwrap_or(start);
                // Expect a string literal
                if let TokenKind::Str = self.tokens.current_kind()? {
                    let tok = self.tokens.advance()?;
                    Some(Expr::Literal(Loc::new(
                        (start, end),
                        Literal::TypedString {
                            data_type,
                            value: tok.span,
                        },
                    )))
                } else {
                    None
                }
            }

            // CASE expression
            TokenKind::Keyword(Keyword::Case) => {
                self.tokens.advance()?;
                self.parse_case_expr()
            }

            // ARRAY[...] constructor
            TokenKind::Keyword(Keyword::Array) => {
                self.tokens.advance()?;
                // Expect [ ... ]
                self.eat(TokenKind::LeftBracket)?;
                let items = if self.tokens.current_kind() == Some(TokenKind::RightBracket) {
                    DelimitedList::default()
                } else {
                    self.parse_list1(TokenKind::Comma, |s| s.parse_expr())?
                };
                self.eat(TokenKind::RightBracket);
                Some(Expr::Array(items))
            }

            // Quantified expressions: ANY(...), SOME(...), ALL(...)
            TokenKind::Keyword(kw @ (Keyword::Any | Keyword::Some | Keyword::All)) => {
                let start = self.tokens.current()?.span.start;
                self.tokens.advance()?;
                self.eat(TokenKind::LeftParen)?;
                let inner = Box::new(self.node(|s| s.parse_expr())?);
                self.eat(TokenKind::RightParen);
                let end = self.tokens.prev().map(|t| t.span.end).unwrap_or(start);
                let quantifier = match kw {
                    Keyword::Any => Quantifier::Any,
                    Keyword::Some => Quantifier::Some,
                    Keyword::All => Quantifier::All,
                    _ => unreachable!(),
                };
                Some(Expr::Quantified(Loc::new(
                    (start, end),
                    QuantifiedExpr {
                        quantifier,
                        expr: inner,
                    },
                )))
            }

            // EXISTS(subquery) - now an operator
            TokenKind::Operator(op) if op.semantic_tag == OpTag::Exists => {
                self.tokens.advance()?;
                self.eat(TokenKind::LeftParen)?;
                let query = Box::new(self.node(|s| s.parse_query())?);
                self.eat(TokenKind::RightParen);
                Some(Expr::Exists(query))
            }

            // Identifiers/columns or function calls
            TokenKind::Identifier
            | TokenKind::IdentifierQuoted(_)
            | TokenKind::Operator(Operator {
                semantic_tag: OpTag::Mul,
                ..
            }) => {
                let qname = self.node(|s| s.parse_qname())?;

                // Check if it's a function call
                if self.tokens.current_kind() == Some(TokenKind::LeftParen) {
                    self.tokens.advance()?;

                    // Check for DISTINCT
                    let distinct = self.eat_kw(Keyword::Distinct).is_some();

                    // Parse arguments
                    let start = self.tokens.prev().map(|t| t.span.end).unwrap_or(0);
                    let args = if self.tokens.current_kind() == Some(TokenKind::RightParen) {
                        DelimitedList::default()
                    } else {
                        self.parse_list1(TokenKind::Comma, |s| s.parse_expr())?
                    };
                    let end = self.tokens.current().map(|t| t.span.start).unwrap_or(start);
                    let args = Loc::new((start, end), args);

                    self.eat(TokenKind::RightParen);

                    // Optional FILTER (WHERE ...)
                    let filter = if self.tokens.current_kind()
                        == Some(TokenKind::Keyword(Keyword::Filter))
                    {
                        self.tokens.advance()?;
                        self.eat(TokenKind::LeftParen)?;
                        self.eat_kw(Keyword::Where)?;
                        let pred = self.node(|s| s.parse_expr())?;
                        self.eat(TokenKind::RightParen);
                        Some(Box::new(pred))
                    } else {
                        None
                    };

                    // Check for OVER clause (window function)
                    if self.tokens.current_kind() == Some(TokenKind::Keyword(Keyword::Over)) {
                        self.tokens.advance()?;
                        // Either a named window or a spec in parens
                        let over = if self.eat(TokenKind::LeftParen).is_some() {
                            let spec = Box::new(self.node(|s| s.parse_window_spec())?);
                            self.eat(TokenKind::RightParen);
                            WindowRef::Spec(spec)
                        } else {
                            // Window name identifier
                            let name_ident = match self.tokens.current_kind()? {
                                TokenKind::Identifier
                                | TokenKind::IdentifierQuoted(_)
                                | TokenKind::Keyword(_) => {
                                    let span = self.tokens.current()?.span;
                                    self.tokens.advance()?;
                                    span
                                }
                                _ => return None,
                            };
                            WindowRef::Name(Loc::new(name_ident, name_ident))
                        };
                        let end_pos = self
                            .tokens
                            .prev()
                            .map(|t| t.span.end)
                            .unwrap_or(qname.span.end);
                        Some(Expr::Over(Loc::new(
                            (qname.span.start, end_pos),
                            OverExpr {
                                name: qname,
                                args,
                                over,
                                filter,
                            },
                        )))
                    } else {
                        let end_pos = self
                            .tokens
                            .prev()
                            .map(|t| t.span.end)
                            .unwrap_or(qname.span.end);
                        Some(Expr::FunctionCall(Loc::new(
                            (qname.span.start, end_pos),
                            FunctionCallExpr {
                                name: qname,
                                distinct,
                                args,
                                filter,
                            },
                        )))
                    }
                } else {
                    Some(Expr::Name(qname))
                }
            }

            // Parenthesized expressions or subqueries
            TokenKind::LeftParen => {
                let open_tok = self.tokens.advance()?;
                let open_span = open_tok.span;

                // Check if it's a subquery
                if let Some(TokenKind::Keyword(Keyword::Select | Keyword::With)) =
                    self.tokens.current_kind()
                {
                    let query = self.node(|s| s.parse_query())?;
                    let _close = self.eat(TokenKind::RightParen);
                    Some(Expr::Subquery(Box::new(query)))
                } else {
                    // Regular parenthesized expression
                    let expr = Box::new(self.node(|s| s.parse_expr())?);
                    let close_span = if self.eat(TokenKind::RightParen).is_some() {
                        self.tokens.prev().map(|t| t.span)
                    } else {
                        None
                    };
                    let end_pos = close_span.map(|s| s.end).unwrap_or(expr.span.end);
                    Some(Expr::Paren(Loc::new(
                        (open_span.start, end_pos),
                        ParenExpr {
                            open: open_span,
                            expr,
                            close: close_span,
                        },
                    )))
                }
            }

            // Unary operators (including NOT)
            TokenKind::Operator(op) if self.is_prefix_op(op.semantic_tag) => {
                let start_pos = self.tokens.current()?.span.start;
                let op_tag = op.semantic_tag;
                self.tokens.advance()?;
                let op_node = self.node(|_| Some(op_tag))?;
                let prec = self.prefix_binding_power(op_tag);
                let expr = Box::new(self.node(|s| s.parse_expr_prec(prec))?);
                let end_pos = expr.span.end;
                Some(Expr::Unary(Loc::new(
                    (start_pos, end_pos),
                    UnaryExpr {
                        op_tok: op_node,
                        expr,
                    },
                )))
            }

            _ => None,
        }
    }

    fn parse_infix(&mut self, left: Expr, start_pos: usize, _prec: u8) -> Option<Expr> {
        let TokenKind::Operator(op) = self.tokens.current_kind()? else {
            return None;
        };

        // Dispatch to specialized handlers for operators with custom syntax
        match op.semantic_tag {
            OpTag::Between => self.parse_between_op(left, start_pos),
            OpTag::Like | OpTag::Ilike => self.parse_like_op(left, start_pos, op.semantic_tag),
            OpTag::Similar => self.parse_similar_op(left, start_pos),
            OpTag::In => self.parse_in_op(left, start_pos),
            OpTag::Is => self.parse_is_op(left, start_pos),
            _ => self.parse_binary_op(left, start_pos, op),
        }
    }

    fn parse_binary_op(&mut self, left: Expr, start_pos: usize, op: Operator) -> Option<Expr> {
        let op_tag = op.semantic_tag;
        let op_prec = op.precedence;
        self.eat_op_tag(op_tag)?;

        let right_prec = match op.assoc {
            Assoc::Left => op_prec + 1,
            Assoc::Right => op_prec,
            Assoc::None => op_prec + 1,
        };

        let right = self.node(|s| s.parse_expr_prec(right_prec)).map(Box::new);
        let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);

        Some(Expr::Binary(Loc::new(
            (start_pos, end_pos),
            BinaryExpr {
                left: Box::new(Loc::new((start_pos, end_pos), left)),
                op: Some(op_tag),
                right,
            },
        )))
    }

    fn parse_between_op(&mut self, left: Expr, start_pos: usize) -> Option<Expr> {
        self.eat_op_tag(OpTag::Between)?;

        let low_opt = self.node(|s| s.parse_expr_prec(10));

        // Consume AND between low and high bounds
        if self.tokens.current_operator_tag() == Some(OpTag::And) {
            self.tokens.advance();
        }

        let high_opt = self.node(|s| s.parse_expr_prec(10));
        let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);

        match (low_opt, high_opt) {
            (Some(low), Some(high)) => Some(Expr::Between(Loc::new(
                (start_pos, end_pos),
                BetweenExpr {
                    expr: Box::new(Loc::new((start_pos, end_pos), left)),
                    low: Box::new(low),
                    high: Box::new(high),
                    not: false,
                },
            ))),
            // Partial input fallback
            _ => Some(Expr::Binary(Loc::new(
                (start_pos, end_pos),
                BinaryExpr {
                    left: Box::new(Loc::new((start_pos, end_pos), left)),
                    op: Some(OpTag::Between),
                    right: None,
                },
            ))),
        }
    }

    fn parse_like_op(&mut self, left: Expr, start_pos: usize, op_tag: OpTag) -> Option<Expr> {
        self.eat_op_tag(op_tag)?;

        let pattern = Box::new(self.node(|s| s.parse_expr_prec(10))?);
        let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);
        let expr = Box::new(Loc::new((start_pos, end_pos), left));

        match op_tag {
            OpTag::Like => Some(Expr::Like(Loc::new(
                (start_pos, end_pos),
                LikeExpr {
                    expr,
                    pattern,
                    not: false,
                },
            ))),
            OpTag::Ilike => Some(Expr::ILike(Loc::new(
                (start_pos, end_pos),
                ILikeExpr {
                    expr,
                    pattern,
                    not: false,
                },
            ))),
            _ => unreachable!(),
        }
    }

    fn parse_similar_op(&mut self, left: Expr, start_pos: usize) -> Option<Expr> {
        self.eat_op_tag(OpTag::Similar)?;
        self.eat_kw(Keyword::To)?;

        let pattern = Box::new(self.node(|s| s.parse_expr_prec(10))?);
        let escape = if self.eat_kw(Keyword::Escape).is_some() {
            Some(Box::new(self.node(|s| s.parse_expr_prec(10))?))
        } else {
            None
        };

        let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);

        Some(Expr::Similar(Loc::new(
            (start_pos, end_pos),
            SimilarExpr {
                expr: Box::new(Loc::new((start_pos, end_pos), left)),
                pattern,
                escape,
            },
        )))
    }

    fn parse_in_op(&mut self, left: Expr, start_pos: usize) -> Option<Expr> {
        self.eat_op_tag(OpTag::In)?;
        self.eat(TokenKind::LeftParen)?;

        let list = if matches!(
            self.tokens.current_kind(),
            Some(TokenKind::Keyword(Keyword::Select | Keyword::With))
        ) {
            let query = Box::new(self.node(|s| s.parse_query())?);
            ExprList::Subquery(query)
        } else {
            let exprs = if self.tokens.current_kind() == Some(TokenKind::RightParen) {
                vec![]
            } else {
                self.parse_list1(TokenKind::Comma, |s| s.parse_expr())?
                    .items
            };
            ExprList::Exprs(exprs)
        };

        self.eat(TokenKind::RightParen);
        let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);

        Some(Expr::In(Loc::new(
            (start_pos, end_pos),
            InExpr {
                expr: Box::new(Loc::new((start_pos, end_pos), left)),
                list,
                not: false,
            },
        )))
    }

    fn parse_is_op(&mut self, left: Expr, start_pos: usize) -> Option<Expr> {
        self.eat_op_tag(OpTag::Is)?;

        let not = if let Some(TokenKind::Operator(op)) = self.tokens.current_kind() {
            if op.semantic_tag == OpTag::Not {
                self.tokens.advance();
                true
            } else {
                false
            }
        } else {
            false
        };

        self.eat_kw(Keyword::Null)?;
        let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);

        Some(Expr::IsNull(Loc::new(
            (start_pos, end_pos),
            IsNullExpr {
                expr: Box::new(Loc::new((start_pos, end_pos), left)),
                not,
            },
        )))
    }

    fn parse_case_expr(&mut self) -> Option<Expr> {
        let start_pos = self.tokens.prev().map(|t| t.span.start).unwrap_or(0);

        // Check for operand (CASE <expr> WHEN ...)
        let operand = if self.tokens.current_kind() != Some(TokenKind::Keyword(Keyword::When)) {
            Some(Box::new(self.node(|s| s.parse_expr())?))
        } else {
            None
        };

        // Parse WHEN clauses
        let mut when_clauses = Vec::new();
        while self.eat_kw(Keyword::When).is_some() {
            let when = self.node(|s| s.parse_expr())?;
            self.eat_kw(Keyword::Then)?;
            let then = self.node(|s| s.parse_expr())?;
            when_clauses.push(WhenClause { when, then });
        }

        // Parse optional ELSE clause
        let else_clause = if self.eat_kw(Keyword::Else).is_some() {
            Some(Box::new(self.node(|s| s.parse_expr())?))
        } else {
            None
        };

        // Expect END keyword
        self.eat_kw(Keyword::End)?;
        let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);

        Some(Expr::Case(Loc::new(
            (start_pos, end_pos),
            CaseExpr {
                operand,
                when_clauses,
                else_clause,
            },
        )))
    }

    // ================ Helper Methods ================

    /// Get the binding power for infix/postfix operators
    fn infix_binding_power(&mut self) -> Option<u8> {
        match self.tokens.current_kind()? {
            TokenKind::Operator(op) => Some(op.precedence),
            _ => None,
        }
    }

    /// Get the binding power for prefix operators
    fn prefix_binding_power(&self, op: OpTag) -> u8 {
        match op {
            OpTag::Not => 20,
            OpTag::Sub | OpTag::Add => 50,
            _ => 50,
        }
    }

    /// Check if an operator can be used as prefix (unary)
    fn is_prefix_op(&self, op: OpTag) -> bool {
        matches!(
            op,
            OpTag::Not | OpTag::Sub | OpTag::Add | OpTag::BitAnd | OpTag::BitOr | OpTag::BitXor
        )
    }
}
