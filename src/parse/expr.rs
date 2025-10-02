use crate::parse::ast::{Literal, Node};
use crate::parse::{Parser, ast::Expr};
use crate::tokenize::{Keyword, OpTag, Operator, TokenKind};

/// Pratt parser for expressions.
impl<'txt> Parser<'txt> {
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
        match self.tokens.current_kind()? {
            // Literals
            TokenKind::Number => {
                let tok = self.tokens.advance()?;
                let val = tok.text.parse::<i64>().ok()?;
                Some(Expr::Literal(Literal::Number(val)))
            }
            TokenKind::Float => {
                let tok = self.tokens.advance()?;
                let val = tok.text.parse::<f64>().ok()?;
                Some(Expr::Literal(Literal::Float(val)))
            }
            TokenKind::Str => {
                let tok = self.tokens.advance()?;
                Some(Expr::Literal(Literal::String(tok.span)))
            }
            TokenKind::Keyword(Keyword::Null) => {
                self.tokens.advance()?;
                Some(Expr::Literal(Literal::Null))
            }
            TokenKind::Keyword(kw @ (Keyword::True | Keyword::False | Keyword::Unknown)) => {
                self.tokens.advance()?;
                let b = match kw {
                    Keyword::True => super::ast::Boolean::True,
                    Keyword::False => super::ast::Boolean::False,
                    Keyword::Unknown => super::ast::Boolean::Unknown,
                    _ => unreachable!(),
                };
                Some(Expr::Literal(Literal::Boolean(b)))
            }

            // Typed literals: DATE 'string', TIME 'string', TIMESTAMP 'string'
            TokenKind::Keyword(kw @ (Keyword::Date | Keyword::Time | Keyword::Timestamp)) => {
                let data_type = match kw {
                    Keyword::Date => super::ast::TypedLiteralKind::Date,
                    Keyword::Time => super::ast::TypedLiteralKind::Time,
                    Keyword::Timestamp => super::ast::TypedLiteralKind::Timestamp,
                    _ => unreachable!(),
                };
                self.tokens.advance()?;

                // Expect a string literal
                if let TokenKind::Str = self.tokens.current_kind()? {
                    let tok = self.tokens.advance()?;
                    Some(Expr::Literal(Literal::TypedString {
                        data_type,
                        value: tok.span,
                    }))
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
                    crate::parse::ast::DelimitedList::default()
                } else {
                    self.parse_list1(TokenKind::Comma, |s| s.parse_expr())?
                };
                self.eat(TokenKind::RightBracket);
                Some(Expr::Array(items))
            }

            // Quantified expressions: ANY(...), SOME(...), ALL(...)
            TokenKind::Keyword(kw @ (Keyword::Any | Keyword::Some | Keyword::All)) => {
                self.tokens.advance()?;
                self.eat(TokenKind::LeftParen)?;
                let inner = Box::new(self.node(|s| s.parse_expr())?);
                self.eat(TokenKind::RightParen);
                let quantifier = match kw {
                    Keyword::Any => super::ast::Quantifier::Any,
                    Keyword::Some => super::ast::Quantifier::Some,
                    Keyword::All => super::ast::Quantifier::All,
                    _ => unreachable!(),
                };
                Some(Expr::Quantified {
                    quantifier,
                    expr: inner,
                })
            }

            // Identifiers/columns or function calls
            TokenKind::Identifier
            | TokenKind::IdentifierQuoted(_)
            | TokenKind::Operator(Operator {
                semantic_tag: OpTag::Mul,
                ..
            }) => {
                let mut qname = self.node(|s| s.parse_qname())?;

                // If parse_qname stopped after a trailing dot, try to capture the next part
                if qname.item.parts.seps.len() >= qname.item.parts.items.len() {
                    match self.tokens.current_kind() {
                        Some(TokenKind::Identifier) | Some(TokenKind::IdentifierQuoted(_)) => {
                            if let Some(tok) = self.tokens.advance() {
                                let span = tok.span;
                                qname.item.parts.items.push(crate::parse::ast::Node::new(
                                    span,
                                    super::ast::NamePart::Ident(span),
                                ));
                            }
                        }
                        Some(TokenKind::Keyword(_)) => {
                            if let Some(tok) = self.tokens.advance() {
                                let span = tok.span;
                                qname.item.parts.items.push(crate::parse::ast::Node::new(
                                    span,
                                    super::ast::NamePart::Ident(span),
                                ));
                            }
                        }
                        _ => {}
                    }
                }

                // Extend qualified name with keyword parts after subsequent dots (e.g., j.key.more)
                while self.eat(TokenKind::Dot).is_some() {
                    // Record the dot separator
                    if let Some(prev) = self.tokens.prev() {
                        qname
                            .item
                            .parts
                            .seps
                            .push(crate::parse::ast::Node::new(prev.span, TokenKind::Dot));
                    }
                    // Accept identifiers, quoted identifiers, or keywords as name parts
                    match self.tokens.current_kind() {
                        Some(TokenKind::Identifier) | Some(TokenKind::IdentifierQuoted(_)) => {
                            if let Some(tok) = self.tokens.advance() {
                                let span = tok.span;
                                qname.item.parts.items.push(crate::parse::ast::Node::new(
                                    span,
                                    super::ast::NamePart::Ident(span),
                                ));
                            } else {
                                break;
                            }
                        }
                        Some(TokenKind::Keyword(_)) => {
                            // Treat keyword as identifier part
                            if let Some(tok) = self.tokens.advance() {
                                let span = tok.span;
                                qname.item.parts.items.push(crate::parse::ast::Node::new(
                                    span,
                                    super::ast::NamePart::Ident(span),
                                ));
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                }

                // Check if it's a function call
                if self.tokens.current_kind() == Some(TokenKind::LeftParen) {
                    self.tokens.advance()?;

                    // Check for DISTINCT
                    let distinct = self.eat_kw(Keyword::Distinct).is_some();

                    // Parse arguments
                    let args = if self.tokens.current_kind() == Some(TokenKind::RightParen) {
                        crate::parse::ast::DelimitedList::default()
                    } else {
                        self.parse_list1(TokenKind::Comma, |s| s.parse_expr())?
                    };

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
                            super::ast::WindowOver::Spec(spec)
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
                            super::ast::WindowOver::Name(Node::new(name_ident, name_ident))
                        };
                        Some(Expr::WindowFunction {
                            name: qname,
                            args,
                            over,
                            filter,
                        })
                    } else {
                        Some(Expr::FunctionCall {
                            name: qname,
                            distinct,
                            args,
                            filter,
                        })
                    }
                } else {
                    Some(Expr::Column(qname))
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
                    Some(Expr::Paren {
                        open: open_span,
                        expr,
                        close: close_span,
                    })
                }
            }

            // Unary operators
            TokenKind::Operator(op) if self.is_prefix_op(op.semantic_tag) => {
                let op_tag = op.semantic_tag;
                self.tokens.advance()?;
                let op_node = self.node(|_| Some(op_tag))?;
                let prec = self.prefix_binding_power(op_tag);
                let expr = Box::new(self.node(|s| s.parse_expr_prec(prec))?);
                Some(Expr::Unary {
                    op_tok: op_node,
                    expr,
                })
            }

            // NOT as prefix
            TokenKind::Keyword(Keyword::Not) => {
                self.tokens.advance()?;
                let op_node = self.node(|_| Some(OpTag::Not))?;
                let prec = self.prefix_binding_power(OpTag::Not);
                let expr = Box::new(self.node(|s| s.parse_expr_prec(prec))?);
                Some(Expr::Unary {
                    op_tok: op_node,
                    expr,
                })
            }

            _ => None,
        }
    }

    fn parse_infix(&mut self, left: Expr, start_pos: usize, _prec: u8) -> Option<Expr> {
        use super::ast::node;

        match self.tokens.current_kind()? {
            // Binary operators
            TokenKind::Operator(op) => {
                let op_tag = op.semantic_tag;
                let op_prec = op.precedence;
                self.tokens.advance()?;

                let right_prec = match op.assoc {
                    crate::tokenize::Assoc::Left => op_prec + 1,
                    crate::tokenize::Assoc::Right => op_prec,
                    crate::tokenize::Assoc::None => op_prec + 1,
                };

                let right = self.node(|s| s.parse_expr_prec(right_prec)).map(Box::new);
                let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);
                Some(Expr::Binary {
                    left: Box::new(node((start_pos, end_pos), left)),
                    op: Some(op_tag),
                    right,
                })
            }

            // IS [NOT] NULL
            TokenKind::Keyword(Keyword::Is) => {
                self.tokens.advance()?;
                let not = self.eat_kw(Keyword::Not).is_some();
                self.eat_kw(Keyword::Null)?;
                let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);
                Some(Expr::IsNull {
                    expr: Box::new(node((start_pos, end_pos), left)),
                    not,
                })
            }

            // [NOT] BETWEEN ... AND ...
            TokenKind::Keyword(Keyword::Between) => {
                // Consume BETWEEN keyword
                self.tokens.advance()?;

                // Try to parse the low and high bounds, but tolerate partial input
                let low_opt = self.node(|s| s.parse_expr_prec(10));

                // Skip AND operator between low and high if present
                if let Some(OpTag::And) = self.tokens.current_operator_tag() {
                    self.tokens.advance();
                }

                let high_opt = self.node(|s| s.parse_expr_prec(10));

                let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);

                match (low_opt, high_opt) {
                    (Some(low), Some(high)) => Some(Expr::Between {
                        expr: Box::new(node((start_pos, end_pos), left)),
                        low: Box::new(low),
                        high: Box::new(high),
                        not: false,
                    }),
                    // On partial input like "a BETWEEN" or "a BETWEEN x AND",
                    // fall back to a Binary node with BETWEEN tag and no RHS so
                    // pretty-printing can preserve the partial text ("a BETWEEN ").
                    _ => Some(Expr::Binary {
                        left: Box::new(node((start_pos, end_pos), left)),
                        op: Some(OpTag::Between),
                        right: None,
                    }),
                }
            }

            // [NOT] LIKE
            TokenKind::Keyword(Keyword::Like) => {
                self.tokens.advance()?;
                let pattern = Box::new(self.node(|s| s.parse_expr_prec(10))?);
                let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);
                Some(Expr::Like {
                    expr: Box::new(node((start_pos, end_pos), left)),
                    pattern,
                    not: false,
                })
            }
            // ILIKE (Postgres)
            TokenKind::Keyword(Keyword::ILike) => {
                self.tokens.advance()?;
                let pattern = Box::new(self.node(|s| s.parse_expr_prec(10))?);
                let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);
                Some(Expr::ILike {
                    expr: Box::new(node((start_pos, end_pos), left)),
                    pattern,
                    not: false,
                })
            }
            // SIMILAR TO [ESCAPE]
            TokenKind::Keyword(Keyword::Similar) => {
                self.tokens.advance()?;
                self.eat_kw(Keyword::To)?;
                let pattern = Box::new(self.node(|s| s.parse_expr_prec(10))?);
                // Optional ESCAPE clause
                let escape = if self.eat_kw(Keyword::Escape).is_some() {
                    Some(Box::new(self.node(|s| s.parse_expr_prec(10))?))
                } else {
                    None
                };
                let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);
                Some(Expr::Similar {
                    expr: Box::new(node((start_pos, end_pos), left)),
                    pattern,
                    escape,
                })
            }

            // [NOT] IN
            TokenKind::Keyword(Keyword::In) => {
                self.tokens.advance()?;
                self.eat(TokenKind::LeftParen)?;

                // Check if it's a subquery or expression list
                let list = if matches!(
                    self.tokens.current_kind(),
                    Some(TokenKind::Keyword(Keyword::Select | Keyword::With))
                ) {
                    // Subquery
                    let query = Box::new(self.node(|s| s.parse_query())?);
                    super::ast::InList::Subquery(query)
                } else {
                    // Expression list
                    let exprs = if self.tokens.current_kind() == Some(TokenKind::RightParen) {
                        vec![]
                    } else {
                        let list = self.parse_list1(TokenKind::Comma, |s| s.parse_expr())?;
                        list.items
                    };
                    super::ast::InList::Exprs(exprs)
                };

                self.eat(TokenKind::RightParen);
                let end_pos = self.tokens.prev().map(|t| t.span.end).unwrap_or(start_pos);
                Some(Expr::In {
                    expr: Box::new(node((start_pos, end_pos), left)),
                    list,
                    not: false,
                })
            }

            _ => None,
        }
    }

    /// Get the binding power for infix/postfix operators
    fn infix_binding_power(&mut self) -> Option<u8> {
        match self.tokens.current_kind()? {
            TokenKind::Operator(op) => Some(op.precedence),
            TokenKind::Keyword(Keyword::Is) => Some(15),
            TokenKind::Keyword(Keyword::Between) => Some(10),
            TokenKind::Keyword(Keyword::Like) => Some(10),
            TokenKind::Keyword(Keyword::ILike) => Some(10),
            TokenKind::Keyword(Keyword::Similar) => Some(10),
            TokenKind::Keyword(Keyword::In) => Some(10),
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

    fn parse_case_expr(&mut self) -> Option<Expr> {
        use super::ast::WhenClause;

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

        Some(Expr::Case {
            operand,
            when_clauses,
            else_clause,
        })
    }
}
