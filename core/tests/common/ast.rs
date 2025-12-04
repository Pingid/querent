use querent_core::ast;
use querent_core::lex::OpTag;
use querent_core::span::Loc;

#[allow(dead_code)]
pub trait AstDisplay {
    fn display(&self, input: &str) -> String;
}

impl AstDisplay for ast::Statement {
    fn display(&self, input: &str) -> String {
        match self {
            ast::Statement::Query(query) => query.item.display(input),
            ast::Statement::Partial(_) => String::new(),
        }
    }
}

impl AstDisplay for ast::Select {
    fn display(&self, input: &str) -> String {
        let mut result = String::from("SELECT");
        match &self.distinct {
            ast::SetQuantifier::All => {}
            ast::SetQuantifier::Distinct => result.push_str(" DISTINCT"),
            ast::SetQuantifier::DistinctOn(list) => {
                let on_items = list
                    .items
                    .iter()
                    .map(|e| e.item.display(input))
                    .collect::<Vec<_>>()
                    .join(", ");
                result.push_str(&format!(" DISTINCT ON ({})", on_items));
            }
        }
        result.push_str(&format!(" {}", self.projection.display(input)));
        if let Some(from) = &self.from {
            result.push_str(&format!(" FROM {}", from.item.display(input)));
        }
        if let Some(where_clause) = &self.where_clause {
            result.push_str(&format!(
                " WHERE {}",
                where_clause.item.expr.item.display(input)
            ));
        }
        if let Some(group_by) = &self.group_by {
            result.push_str(&format!(" GROUP BY {}", group_by.item.display(input)));
        }
        if let Some(having) = &self.having {
            result.push_str(&format!(" HAVING {}", having.item.display(input)));
        }
        if let Some(window) = &self.window {
            result.push_str(&format!(" WINDOW {}", window.item.display(input)));
        }
        result
    }
}

impl AstDisplay for ast::Projection {
    fn display(&self, input: &str) -> String {
        self.list
            .items
            .iter()
            .map(|item| item.item.display(input))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl AstDisplay for ast::ProjectionItem {
    fn display(&self, input: &str) -> String {
        if let Some(alias) = &self.alias {
            format!(
                "{} AS {}",
                self.expr.item.display(input),
                alias.span.as_str(input)
            )
        } else {
            self.expr.item.display(input)
        }
    }
}

impl AstDisplay for ast::Expr {
    fn display(&self, input: &str) -> String {
        match self {
            ast::Expr::Literal(literal) => literal.display(input),
            ast::Expr::Binary(b) => {
                let op_str = match b.op.map(|o| o.item) {
                    Some(OpTag::Add) => " + ",
                    Some(OpTag::Sub) => " - ",
                    Some(OpTag::Mul) => " * ",
                    Some(OpTag::Div) => " / ",
                    Some(OpTag::Mod) => " % ",
                    Some(OpTag::Eq) => " = ",
                    Some(OpTag::Neq) => " != ",
                    Some(OpTag::Lt) => " < ",
                    Some(OpTag::Lte) => " <= ",
                    Some(OpTag::Gt) => " > ",
                    Some(OpTag::Gte) => " >= ",
                    Some(OpTag::And) => " AND ",
                    Some(OpTag::Or) => " OR ",
                    Some(OpTag::Like) => " LIKE ",
                    Some(OpTag::Concat) => " || ",
                    Some(OpTag::Regex) => " ~ ",
                    Some(OpTag::NotRegex) => " !~ ",
                    Some(OpTag::RegexI) => " ~* ",
                    Some(OpTag::NotRegexI) => " !~* ",
                    Some(OpTag::Between) => " BETWEEN ",
                    Some(OpTag::In) => " IN ",
                    Some(OpTag::Overlap) => " && ",
                    Some(OpTag::Overlaps) => " OVERLAPS ",
                    Some(OpTag::Contains) => " @> ",
                    Some(OpTag::ContainedBy) => " <@ ",
                    Some(OpTag::JsonGet) => "->",
                    Some(OpTag::JsonGetText) => "->>",
                    Some(OpTag::JsonPath) => "#>",
                    Some(OpTag::JsonPathText) => "#>>",
                    Some(OpTag::JsonKeyExists) => " ? ",
                    Some(OpTag::JsonAnyKey) => " ?| ",
                    Some(OpTag::JsonAllKeys) => " ?& ",
                    Some(OpTag::JsonPathMatch) => " @? ",
                    Some(OpTag::JsonPathBool) => " @@ ",
                    Some(OpTag::BitAnd) => " & ",
                    Some(OpTag::BitOr) => " | ",
                    Some(OpTag::BitXor) => " # ",
                    Some(OpTag::Shl) => " << ",
                    Some(OpTag::Shr) => " >> ",
                    Some(OpTag::Exp) => " ^ ",
                    Some(OpTag::TypeCast) => "::",
                    _ => " ",
                };
                format!(
                    "{}{}{}",
                    b.left.item.display(input),
                    op_str,
                    b.right
                        .as_ref()
                        .map(|r| r.item.display(input))
                        .unwrap_or_default()
                )
            }
            ast::Expr::Name(name) => name.item.display(input),
            ast::Expr::Paren(p) => {
                format!("({})", p.expr.item.display(input))
            }
            ast::Expr::Unary(u) => {
                let op_str = match u.op_tok.item {
                    OpTag::Not => "NOT ",
                    OpTag::Sub | OpTag::UnaryMinus => "-",
                    OpTag::Add | OpTag::UnaryPlus => "+",
                    _ => "",
                };
                format!("{}{}", op_str, u.expr.item.display(input))
            }
            ast::Expr::IsNull(isn) => {
                if isn.not {
                    format!("{} IS NOT NULL", isn.expr.item.display(input))
                } else {
                    format!("{} IS NULL", isn.expr.item.display(input))
                }
            }
            ast::Expr::Between(b) => {
                if b.not {
                    format!(
                        "{} NOT BETWEEN {} AND {}",
                        b.expr.item.display(input),
                        b.low.item.display(input),
                        b.high.item.display(input)
                    )
                } else {
                    format!(
                        "{} BETWEEN {} AND {}",
                        b.expr.item.display(input),
                        b.low.item.display(input),
                        b.high.item.display(input)
                    )
                }
            }
            ast::Expr::Like(l) => {
                if l.not {
                    format!(
                        "{} NOT LIKE {}",
                        l.expr.item.display(input),
                        l.pattern.item.display(input)
                    )
                } else {
                    format!(
                        "{} LIKE {}",
                        l.expr.item.display(input),
                        l.pattern.item.display(input)
                    )
                }
            }
            ast::Expr::ILike(il) => {
                if il.not {
                    format!(
                        "{} NOT ILIKE {}",
                        il.expr.item.display(input),
                        il.pattern.item.display(input)
                    )
                } else {
                    format!(
                        "{} ILIKE {}",
                        il.expr.item.display(input),
                        il.pattern.item.display(input)
                    )
                }
            }
            ast::Expr::Similar(s) => {
                let mut result = format!(
                    "{} SIMILAR TO {}",
                    s.expr.item.display(input),
                    s.pattern.item.display(input)
                );
                if let Some(esc) = &s.escape {
                    result.push_str(&format!(" ESCAPE {}", esc.item.display(input)));
                }
                result
            }
            ast::Expr::FunctionCall(fc) => {
                let distinct_str = if fc.distinct { "DISTINCT " } else { "" };
                let args_str = fc
                    .args
                    .items
                    .iter()
                    .map(|arg| arg.item.display(input))
                    .collect::<Vec<_>>()
                    .join(",");
                let mut s = format!(
                    "{}({}{})",
                    fc.name.item.display(input),
                    distinct_str,
                    args_str
                );
                if let Some(f) = &fc.filter {
                    s.push_str(&format!(" FILTER (WHERE {})", f.item.display(input)));
                }
                s
            }
            ast::Expr::Array(items) => {
                let inner = items
                    .items
                    .iter()
                    .map(|e| e.item.display(input))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("ARRAY[{}]", inner)
            }
            ast::Expr::Quantified(q) => {
                let quantifier_str = match q.quantifier {
                    ast::Quantifier::Any => "ANY",
                    ast::Quantifier::Some => "SOME",
                    ast::Quantifier::All => "ALL",
                };
                format!("{}({})", quantifier_str, q.expr.item.display(input))
            }
            ast::Expr::Exists(query) => {
                format!("EXISTS ({})", query.item.display(input))
            }
            ast::Expr::Subquery(query) => {
                format!("({})", query.item.display(input))
            }
            ast::Expr::Case(c) => {
                let mut result = String::from("CASE");

                if let Some(operand) = &c.operand {
                    result.push_str(&format!(" {}", operand.item.display(input)));
                }

                for clause in &c.when_clauses {
                    result.push_str(&format!(
                        " WHEN {} THEN {}",
                        clause.when.item.display(input),
                        clause.then.item.display(input)
                    ));
                }

                if let Some(else_expr) = &c.else_clause {
                    result.push_str(&format!(" ELSE {}", else_expr.item.display(input)));
                }

                result.push_str(" END");
                result
            }
            ast::Expr::In(i) => {
                let mut result = i.expr.item.display(input);
                if i.not {
                    result.push_str(" NOT IN (");
                } else {
                    result.push_str(" IN (");
                }
                match &i.list {
                    ast::ExprList::Subquery(query) => {
                        result.push_str(&query.item.display(input));
                    }
                    ast::ExprList::Exprs(exprs) => {
                        let expr_strs = exprs
                            .iter()
                            .map(|e| e.item.display(input))
                            .collect::<Vec<_>>()
                            .join(", ");
                        result.push_str(&expr_strs);
                    }
                }
                result.push(')');
                result
            }
            ast::Expr::Over(wf) => {
                let args_str = wf
                    .args
                    .items
                    .iter()
                    .map(|arg| arg.item.display(input))
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut result = format!("{}({})", wf.name.item.display(input), args_str);
                if let Some(f) = &wf.filter {
                    result.push_str(&format!(" FILTER (WHERE {})", f.item.display(input)));
                }
                result.push_str(" OVER ");

                match &wf.over {
                    ast::WindowRef::Name(w) => {
                        result.push_str(w.span.as_str(input));
                        result
                    }
                    ast::WindowRef::Spec(spec) => {
                        result.push('(');
                        let over = spec;

                        if let Some(partition_by) = &over.item.partition_by {
                            let partition_exprs = partition_by
                                .items
                                .iter()
                                .map(|e| e.item.display(input))
                                .collect::<Vec<_>>()
                                .join(", ");
                            result.push_str(&format!("PARTITION BY {}", partition_exprs));
                        }

                        if let Some(order_by) = &over.item.order_by {
                            if over.item.partition_by.is_some() {
                                result.push(' ');
                            }
                            result.push_str(&format!("ORDER BY {}", order_by.item.display(input)));
                        }

                        if let Some(frame) = &over.item.frame {
                            if over.item.order_by.is_some() || over.item.partition_by.is_some() {
                                result.push(' ');
                            }
                            result.push_str(&frame.item.display(input));
                        }

                        result.push(')');
                        result
                    }
                }
            }
            ast::Expr::Cast(cast) => {
                format!(
                    "CAST({} AS {})",
                    cast.expr.item.display(input),
                    cast.data_type.item.display(input)
                )
            }
            ast::Expr::Subscript(sub) => {
                if let Some(upper) = &sub.upper {
                    format!(
                        "{}[{}:{}]",
                        sub.expr.item.display(input),
                        sub.index.item.display(input),
                        upper.item.display(input)
                    )
                } else {
                    format!(
                        "{}[{}]",
                        sub.expr.item.display(input),
                        sub.index.item.display(input)
                    )
                }
            }
            ast::Expr::Row(row) => {
                let exprs = row
                    .exprs
                    .items
                    .iter()
                    .map(|e| e.item.display(input))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("ROW({})", exprs)
            }
            ast::Expr::AtTimeZone(atz) => {
                format!(
                    "{} AT TIME ZONE {}",
                    atz.expr.item.display(input),
                    atz.timezone.item.display(input)
                )
            }
            ast::Expr::Empty => String::new(),
        }
    }
}

impl AstDisplay for ast::DataType {
    fn display(&self, input: &str) -> String {
        match self {
            ast::DataType::Named(name) => name.item.display(input),
            ast::DataType::Parameterized { name, params } => {
                let params_str = params
                    .iter()
                    .map(|p| p.item.display(input))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", name.item.display(input), params_str)
            }
        }
    }
}

impl AstDisplay for ast::TypeParam {
    fn display(&self, input: &str) -> String {
        match self {
            ast::TypeParam::Number(n) => n.to_string(),
            ast::TypeParam::Ident(id) => id.as_str(input).to_string(),
        }
    }
}

impl AstDisplay for ast::Literal {
    fn display(&self, input: &str) -> String {
        match self {
            ast::Literal::Number(number) => number.to_string(),
            ast::Literal::Float(float) => float.to_string(),
            ast::Literal::String(string) => {
                // The span includes the quotes, so just return as-is
                string.as_str(input).to_string()
            }
            ast::Literal::Null => "NULL".to_string(),
            ast::Literal::Boolean(b) => match b {
                ast::Boolean::True => "TRUE".to_string(),
                ast::Boolean::False => "FALSE".to_string(),
                ast::Boolean::Unknown => "UNKNOWN".to_string(),
            },
            ast::Literal::TypedString { data_type, value } => {
                let type_str = match data_type {
                    ast::TypedLiteralKind::Date => "DATE",
                    ast::TypedLiteralKind::Time => "TIME",
                    ast::TypedLiteralKind::Timestamp => "TIMESTAMP",
                    ast::TypedLiteralKind::Interval => "INTERVAL",
                };
                format!("{} {}", type_str, value.as_str(input))
            }
        }
    }
}

impl AstDisplay for ast::QualifiedName {
    fn display(&self, input: &str) -> String {
        self.parts
            .items
            .iter()
            .map(|part| part.item.display(input))
            .collect::<Vec<_>>()
            .join(".")
    }
}

impl AstDisplay for ast::NamePart {
    fn display(&self, input: &str) -> String {
        match self {
            ast::NamePart::Ident(ident) => ident.as_str(input).to_string(),
            ast::NamePart::Star => "*".to_string(),
        }
    }
}

impl AstDisplay for ast::DelimitedList<Loc<ast::ProjectionItem>> {
    fn display(&self, input: &str) -> String {
        self.items
            .iter()
            .map(|item| item.item.display(input))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl AstDisplay for ast::WindowFrame {
    fn display(&self, input: &str) -> String {
        let unit_str = match self.unit {
            ast::FrameUnit::Rows => "ROWS",
            ast::FrameUnit::Range => "RANGE",
            ast::FrameUnit::Groups => "GROUPS",
        };
        let mut result = format!("{} BETWEEN {}", unit_str, self.start.item.display(input));
        if let Some(end) = &self.end {
            result.push_str(&format!(" AND {}", end.item.display(input)));
        }
        result
    }
}

impl AstDisplay for ast::WindowSpec {
    fn display(&self, input: &str) -> String {
        let mut result = String::new();
        if let Some(partition_by) = &self.partition_by {
            let parts = partition_by
                .items
                .iter()
                .map(|e| e.item.display(input))
                .collect::<Vec<_>>()
                .join(", ");
            result.push_str(&format!("PARTITION BY {}", parts));
        }
        if let Some(order_by) = &self.order_by {
            if self.partition_by.is_some() {
                result.push(' ');
            }
            result.push_str(&format!("ORDER BY {}", order_by.item.display(input)));
        }
        if let Some(frame) = &self.frame {
            if self.partition_by.is_some() || self.order_by.is_some() {
                result.push(' ');
            }
            result.push_str(&frame.item.display(input));
        }
        result
    }
}

impl AstDisplay for ast::Window {
    fn display(&self, input: &str) -> String {
        self.windows
            .iter()
            .map(|w| w.item.display(input))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl AstDisplay for ast::WindowDef {
    fn display(&self, input: &str) -> String {
        format!(
            "{} AS ({})",
            self.name.as_str(input),
            self.spec.item.display(input)
        )
    }
}

impl AstDisplay for ast::FrameBound {
    fn display(&self, input: &str) -> String {
        match self {
            ast::FrameBound::UnboundedPreceding => "UNBOUNDED PRECEDING".to_string(),
            ast::FrameBound::UnboundedFollowing => "UNBOUNDED FOLLOWING".to_string(),
            ast::FrameBound::CurrentRow => "CURRENT ROW".to_string(),
            ast::FrameBound::Preceding(expr) => format!("{} PRECEDING", expr.item.display(input)),
            ast::FrameBound::Following(expr) => format!("{} FOLLOWING", expr.item.display(input)),
        }
    }
}

impl AstDisplay for ast::From {
    fn display(&self, input: &str) -> String {
        self.sources
            .items
            .iter()
            .map(|table| table.item.display(input))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl AstDisplay for ast::TableRef {
    fn display(&self, input: &str) -> String {
        match self {
            ast::TableRef::Factor(factor) => factor.item.display(input),
            ast::TableRef::Join(join) => {
                let mut result = join.left.item.display(input);

                // Add join type
                let natural_prefix = if join.kind.natural { "NATURAL " } else { "" };
                let join_str = match join.kind.base {
                    ast::JoinBase::Inner => {
                        if join.kind.natural {
                            "JOIN"
                        } else {
                            "INNER JOIN"
                        }
                    }
                    ast::JoinBase::Left => {
                        if join.kind.outer {
                            "LEFT OUTER JOIN"
                        } else {
                            "LEFT JOIN"
                        }
                    }
                    ast::JoinBase::Right => {
                        if join.kind.outer {
                            "RIGHT OUTER JOIN"
                        } else {
                            "RIGHT JOIN"
                        }
                    }
                    ast::JoinBase::Full => {
                        if join.kind.outer {
                            "FULL OUTER JOIN"
                        } else {
                            "FULL JOIN"
                        }
                    }
                    ast::JoinBase::Cross => "CROSS JOIN",
                };

                result.push_str(&format!(" {}{} {}", natural_prefix, join_str, join.right.item.display(input)));

                // Add join constraint
                if let Some(constraint) = &join.constraint {
                    match &constraint.item {
                        ast::JoinConstraint::On(expr) => {
                            result.push_str(&format!(" ON {}", expr.item.display(input)));
                        }
                        ast::JoinConstraint::Using(cols) => {
                            let col_names = cols
                                .items
                                .iter()
                                .map(|col| col.span.as_str(input))
                                .collect::<Vec<_>>()
                                .join(", ");
                            result.push_str(&format!(" USING ({})", col_names));
                        }
                    }
                }

                result
            }
        }
    }
}

impl AstDisplay for ast::TableFactor {
    fn display(&self, input: &str) -> String {
        match self {
            ast::TableFactor::Named(n) => {
                let mut name_str = String::new();
                if n.lateral {
                    name_str.push_str("LATERAL ");
                }
                name_str.push_str(&n.name.item.display(input));
                if let Some(alias) = n.alias {
                    format!("{} {}", name_str, alias.span.as_str(input))
                } else {
                    name_str
                }
            }
            ast::TableFactor::Function(f) => {
                let mut result = String::new();
                if f.lateral {
                    result.push_str("LATERAL ");
                }
                let args_str = f
                    .args
                    .items
                    .iter()
                    .map(|a| a.item.display(input))
                    .collect::<Vec<_>>()
                    .join(",");
                result.push_str(&format!("{}({})", f.name.item.display(input), args_str));
                if let Some(alias) = f.alias {
                    result.push_str(&format!(" AS {}", alias.span.as_str(input)));
                    if let Some(cols) = &f.columns {
                        let col_str = cols
                            .items
                            .iter()
                            .map(|c| c.span.as_str(input))
                            .collect::<Vec<_>>()
                            .join(",");
                        result.push_str(&format!("({})", col_str));
                    }
                }
                result
            }
            ast::TableFactor::Subquery(s) => {
                let mut result = String::new();
                // NOTE: LATERAL is not shown here; add if needed
                result.push_str(&format!("({})", s.query.item.display(input)));
                if let Some(alias) = s.alias {
                    result.push_str(&format!(" {}", alias.span.as_str(input)));
                }
                result
            }
            ast::TableFactor::Parenthesized(inner) => {
                format!("({})", inner.item.display(input))
            }
        }
    }
}

impl AstDisplay for ast::Query {
    fn display(&self, input: &str) -> String {
        let mut result = String::new();
        if let Some(with) = &self.with {
            result.push_str("WITH ");
            result.push_str(&with.item.display(input));
            if self.body.is_some() {
                result.push(' ');
            }
        }
        if let Some(body) = &self.body {
            result.push_str(&body.item.display(input));
        }
        if let Some(tail) = &self.tail {
            result.push_str(&tail.item.display(input));
        }
        result
    }
}

impl AstDisplay for ast::With {
    fn display(&self, input: &str) -> String {
        let mut parts: Vec<String> = Vec::new();
        for cte in &self.ctes {
            parts.push(cte.item.display(input));
        }
        let mut s = String::new();
        if self.recursive {
            s.push_str("RECURSIVE ");
        }
        s.push_str(&parts.join(", "));
        s
    }
}

impl AstDisplay for ast::Cte {
    fn display(&self, input: &str) -> String {
        let mut result = String::new();
        result.push_str(self.name.as_str(input));
        if let Some(cols) = &self.columns {
            let cols_str = cols
                .items
                .iter()
                .map(|c| c.span.as_str(input))
                .collect::<Vec<_>>()
                .join(", ");
            result.push_str(&format!(" ({})", cols_str));
        }
        if let Some(mat) = self.materialized {
            match mat {
                ast::CteMaterialization::Materialized => result.push_str(" MATERIALIZED"),
                ast::CteMaterialization::NotMaterialized => result.push_str(" NOT MATERIALIZED"),
            }
        }
        result.push_str(&format!(" AS ({})", self.query.item.display(input)));
        result
    }
}

impl AstDisplay for ast::QuerySuffix {
    fn display(&self, input: &str) -> String {
        let mut result = String::new();
        if let Some(order_by) = &self.order_by {
            result.push_str(&format!(" ORDER BY {}", order_by.item.display(input)));
        }
        match &self.limit {
            Some(l) if matches!(l.item.style, ast::LimitKind::Limit) => {
                // LIMIT style: print LIMIT then optional OFFSET
                result.push_str(&format!(" {}", l.item.display(input)));
                if let Some(offset) = &self.offset {
                    result.push_str(&format!(" {}", offset.item.display(input)));
                }
            }
            _ => {
                // FETCH style or no limit: print OFFSET then LIMIT
                if let Some(offset) = &self.offset {
                    result.push_str(&format!(" {}", offset.item.display(input)));
                }
                if let Some(limit) = &self.limit {
                    result.push_str(&format!(" {}", limit.item.display(input)));
                }
            }
        }
        result
    }
}

impl AstDisplay for ast::OrderBy {
    fn display(&self, input: &str) -> String {
        self.items
            .items
            .iter()
            .map(|item| item.item.display(input))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl AstDisplay for ast::OrderByItem {
    fn display(&self, input: &str) -> String {
        let mut result = self.expr.item.display(input);
        if let Some(direction) = &self.direction {
            match direction {
                ast::SortDirection::Asc => result.push_str(" ASC"),
                ast::SortDirection::Desc => result.push_str(" DESC"),
            }
        }
        if let Some(nulls) = &self.nulls {
            match nulls {
                ast::NullsOrder::First => result.push_str(" NULLS FIRST"),
                ast::NullsOrder::Last => result.push_str(" NULLS LAST"),
            }
        }
        result
    }
}

impl AstDisplay for ast::Limit {
    fn display(&self, input: &str) -> String {
        match self.style {
            ast::LimitKind::FetchFirst => {
                format!("FETCH FIRST {} ROWS ONLY", self.count.item.display(input))
            }
            ast::LimitKind::Limit => format!("LIMIT {}", self.count.item.display(input)),
        }
    }
}

impl AstDisplay for ast::Offset {
    fn display(&self, input: &str) -> String {
        if self.rows_keyword {
            format!("OFFSET {} ROWS", self.count.item.display(input))
        } else {
            format!("OFFSET {}", self.count.item.display(input))
        }
    }
}

impl AstDisplay for ast::QueryExpr {
    fn display(&self, input: &str) -> String {
        let mut result = self.left.item.display(input);

        for set_op_chain in &self.set_ops {
            let op_str = match set_op_chain.op {
                ast::SetOp::Union { all } => {
                    if all {
                        " UNION ALL "
                    } else {
                        " UNION "
                    }
                }
                ast::SetOp::Intersect { all } => {
                    if all {
                        " INTERSECT ALL "
                    } else {
                        " INTERSECT "
                    }
                }
                ast::SetOp::Except { all } => {
                    if all {
                        " EXCEPT ALL "
                    } else {
                        " EXCEPT "
                    }
                }
                ast::SetOp::Minus { all } => {
                    if all {
                        " MINUS ALL "
                    } else {
                        " MINUS "
                    }
                }
            };
            result.push_str(op_str);
            result.push_str(&set_op_chain.right.item.display(input));
        }

        result
    }
}

impl AstDisplay for ast::QueryPrimary {
    fn display(&self, input: &str) -> String {
        match self {
            ast::QueryPrimary::Select(stmt) => stmt.display(input),
            ast::QueryPrimary::Values(_) => String::new(),
            ast::QueryPrimary::Parenthesized(_) => String::new(),
        }
    }
}

impl AstDisplay for ast::GroupBy {
    fn display(&self, input: &str) -> String {
        self.items
            .items
            .iter()
            .map(|item| item.item.display(input))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl AstDisplay for ast::GroupByItem {
    fn display(&self, input: &str) -> String {
        match self {
            ast::GroupByItem::Expr(expr) => expr.item.display(input),
            ast::GroupByItem::Rollup(_) => String::new(),
            ast::GroupByItem::Cube(_) => String::new(),
            ast::GroupByItem::GroupingSets(_) => String::new(),
        }
    }
}
