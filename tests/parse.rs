use querent::{
    dialect::{Dialect, DialectSpec, ansi::AnsiDialect, postgres::PgDialect},
    parse::{Parser, ast::*},
    tokenize::{OpTag, TokenTape, Tokenizer},
};

// ---------------- Test: Statement ----------------
#[test]
fn statement_partial_and_query() {
    let dialect = AnsiDialect::default();
    let spec = dialect.spec();
    let ast = statement("", spec);
    assert!(matches!(ast, Statement::Partial(_)));
    let ast = statement("SEL", spec);
    assert!(matches!(ast, Statement::Partial(_)));
    let ast = statement("SELECT", spec);
    assert!(matches!(ast, Statement::Query(_)));
}

#[test]
fn ans_complete() {
    let inputs = [
        // Literals & expressions
        "SELECT 1",
        "SELECT NULL",
        "SELECT a + b, c * d, e / f, g - h",
        "SELECT (a + b) * c",
        "SELECT CASE WHEN age > 18 THEN 'adult' ELSE 'child' END AS category",
        // DISTINCT / alias
        "SELECT DISTINCT a, b",
        "SELECT a AS alias1, b AS alias2",
        // FROM / WHERE
        "SELECT * FROM users",
        "SELECT id, name FROM users WHERE age > 18",
        "SELECT * FROM users WHERE name LIKE 'A%'",
        "SELECT * FROM users WHERE created_at BETWEEN DATE '2020-01-01' AND DATE '2020-12-31'",
        "SELECT * FROM users WHERE status IS NOT NULL",
        // Joins
        "SELECT u.id, o.id FROM users u INNER JOIN orders o ON u.id = o.user_id",
        "SELECT * FROM a LEFT JOIN b ON a.id = b.a_id",
        "SELECT * FROM a RIGHT JOIN b ON a.id = b.a_id",
        "SELECT * FROM a FULL JOIN b ON a.id = b.a_id",
        "SELECT * FROM t1 CROSS JOIN t2",
        // Grouping / Having
        "SELECT age, COUNT(*) FROM users GROUP BY age",
        "SELECT dept, AVG(salary) FROM employees GROUP BY dept HAVING AVG(salary) > 50000",
        // Ordering
        "SELECT * FROM users ORDER BY name ASC",
        "SELECT * FROM users ORDER BY age DESC, name ASC",
        "SELECT * FROM users OFFSET 20 ROWS FETCH FIRST 10 ROWS ONLY",
        "SELECT id, RANK() OVER (PARTITION BY dept ORDER BY salary DESC) AS r FROM employees",
        "SELECT id, SUM(amount) OVER (ORDER BY created_at ROWS BETWEEN 1 PRECEDING AND CURRENT ROW) FROM orders",
        // Subqueries
        "SELECT * FROM (SELECT id, name FROM users) u",
        "SELECT id, (SELECT COUNT(*) FROM orders o WHERE o.user_id = u.id) AS order_count FROM users u",
        "SELECT * FROM users WHERE id IN (SELECT user_id FROM orders)",
        // Set operations
        "SELECT id FROM users UNION SELECT id FROM admins",
        "SELECT id FROM a INTERSECT SELECT id FROM b",
        "SELECT id FROM a EXCEPT SELECT id FROM b",
        // CTEs
        "WITH cte AS (SELECT id FROM users) SELECT * FROM cte",
    ];
    assert_display_match(&inputs, &ansi_compliant_specs());
}

#[test]
fn ansi_partial() {
    let matching = [
        "SELECT * FROM ",
        "SELECT a FROM t WHERE ",
        "SELECT a FROM t WHERE a = ",
        "SELECT  FROM t",
        "SELECT a FROM t WHERE a BETWEEN ",
    ];
    assert_display_match(&matching, &ansi_compliant_specs());

    let inputs = [
        ("SELECT a, FROM t ", "SELECT a FROM t"),
        // Subqueries & set ops
        ("SELECT * FROM (SELECT", "SELECT * FROM (SELECT )"),
    ];
    assert_display_match_pairs(&inputs, &ansi_compliant_specs());
}

#[test]
fn pg_complete() {
    let inputs = [
        // DISTINCT ON (Postgres extension)
        "SELECT DISTINCT ON (user_id) user_id, created_at FROM events ORDER BY user_id, created_at DESC",
        // ILIKE, regex, SIMILAR TO + ESCAPE
        "SELECT * FROM users WHERE name ILIKE 'al%'",
        "SELECT * FROM users WHERE email ~ '^[A-Za-z0-9._%+-]+@example\\.com$'",
        "SELECT * FROM users WHERE note SIMILAR TO '(foo|bar)%' ESCAPE '\\'",
        // JSONB operators
        "SELECT data->'profile' AS profile FROM accounts",
        "SELECT data->>'email' AS email FROM accounts",
        "SELECT data#>'{address,city}' AS city FROM accounts",
        "SELECT data#>>'{address,city}' AS city_text FROM accounts",
        "SELECT * FROM accounts WHERE data ? 'email'",
        "SELECT * FROM accounts WHERE data ?| ARRAY['email','phone']",
        // Arrays (ANY/SOME/ALL and overlap/containment)
        "SELECT * FROM posts WHERE 'pg' = ANY(tags)",
        "SELECT * FROM posts WHERE tags && ARRAY['pg','db']",
        "SELECT * FROM posts WHERE ARRAY['pg','db'] <@ tags",
        "SELECT * FROM posts WHERE tags @> ARRAY['pg']",
        // DISTINCT ON + LIMIT/OFFSET combo
        "SELECT DISTINCT ON (user_id) * FROM messages ORDER BY user_id, created_at DESC LIMIT 10 OFFSET 20",
        // LATERAL
        "SELECT u.id, x.word FROM users u, LATERAL unnest(string_to_array(u.bio,' ')) AS x(word)",
        "SELECT u.id, j.key, j.value FROM users u LEFT JOIN LATERAL jsonb_each(u.data) AS j(key,value) ON TRUE",
        // WINDOW name + FILTER
        "SELECT dept, SUM(salary) FILTER (WHERE active) OVER w AS active_sum, SUM(salary) OVER w AS total_sum FROM emp WINDOW w AS (PARTITION BY dept ORDER BY hired_at)",
    ];
    let d = PgDialect::default();
    let spec = d.spec();
    assert_display_match(&inputs, &[*spec]);
}

// ---------------- Test utils ----------------
fn statement(sql: &str, s: &DialectSpec) -> Statement {
    let tokenizer = Tokenizer::new(&s, sql);
    let tokens = tokenizer.collect::<Vec<_>>();
    let mut parser = Parser::new(TokenTape::new(tokens));
    parser.parse_statement().unwrap()
}

fn assert_display_match(inputs: &[&str], specs: &[DialectSpec]) {
    for spec in specs {
        for input in inputs {
            let s = statement(input, &spec);
            let output = fmt(input, &s);
            if output != *input {
                eprintln!("Input:  {:?}", input);
                eprintln!("Output: {:?}", output);
                eprintln!("AST:    {:#?}", s);
            }
            assert_eq!(output, input.to_string());
        }
    }
}

fn assert_display_match_pairs(inputs: &[(&str, &str)], specs: &[DialectSpec]) {
    for spec in specs {
        for (input, expected) in inputs {
            let s = statement(input, &spec);
            let output = fmt(input, &s);
            if output != *expected {
                eprintln!("Input:  {:?}", input);
                eprintln!("Output: {:?}", output);
                eprintln!("AST:    {:#?}", s);
            }
            assert_eq!(output, expected.to_string());
        }
    }
}

fn ansi_compliant_specs() -> Vec<DialectSpec> {
    let ansi = AnsiDialect::default();
    let pg = PgDialect::default();
    vec![*ansi.spec(), *pg.spec()]
}

fn fmt<T: Display>(input: &str, display: &T) -> String {
    display.display(input)
}

trait Display {
    fn display(&self, input: &str) -> String;
}

impl Display for Statement {
    fn display(&self, input: &str) -> String {
        match self {
            Statement::Query(query) => query.item.display(input),
            Statement::Partial(_) => String::new(),
        }
    }
}

impl Display for SelectStmt {
    fn display(&self, input: &str) -> String {
        let mut result = String::from("SELECT");
        match &self.distinct {
            Distinct::All => {}
            Distinct::Distinct => result.push_str(" DISTINCT"),
            Distinct::DistinctOn(list) => {
                let on_items = list
                    .items
                    .iter()
                    .map(|e| e.item.display(input))
                    .collect::<Vec<_>>()
                    .join(", ");
                result.push_str(&format!(" DISTINCT ON ({})", on_items));
            }
        }
        result.push_str(&format!(" {}", self.projection.item.display(input)));
        if let Some(from) = &self.from {
            result.push_str(&format!(" FROM {}", from.item.display(input)));
        }
        if let Some(where_clause) = &self.where_clause {
            result.push_str(&format!(" WHERE {}", where_clause.item.display(input)));
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

impl Display for SelectItem {
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

impl Display for Expr {
    fn display(&self, input: &str) -> String {
        match self {
            Expr::Literal(literal) => literal.display(input),
            Expr::Binary { left, op, right } => {
                let op_str = match op {
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
                    _ => " ",
                };
                format!(
                    "{}{}{}",
                    left.item.display(input),
                    op_str,
                    right
                        .as_ref()
                        .map(|r| r.item.display(input))
                        .unwrap_or_default()
                )
            }
            Expr::Column(name) => name.item.display(input),
            Expr::Paren { expr, .. } => {
                format!("({})", expr.item.display(input))
            }
            Expr::Unary { op_tok, expr } => {
                let op_str = match op_tok.item {
                    OpTag::Not => "NOT ",
                    OpTag::Sub => "-",
                    OpTag::Add => "+",
                    _ => "",
                };
                format!("{}{}", op_str, expr.item.display(input))
            }
            Expr::IsNull { expr, not } => {
                if *not {
                    format!("{} IS NOT NULL", expr.item.display(input))
                } else {
                    format!("{} IS NULL", expr.item.display(input))
                }
            }
            Expr::Between {
                expr,
                low,
                high,
                not,
            } => {
                if *not {
                    format!(
                        "{} NOT BETWEEN {} AND {}",
                        expr.item.display(input),
                        low.item.display(input),
                        high.item.display(input)
                    )
                } else {
                    format!(
                        "{} BETWEEN {} AND {}",
                        expr.item.display(input),
                        low.item.display(input),
                        high.item.display(input)
                    )
                }
            }
            Expr::Like { expr, pattern, not } => {
                if *not {
                    format!(
                        "{} NOT LIKE {}",
                        expr.item.display(input),
                        pattern.item.display(input)
                    )
                } else {
                    format!(
                        "{} LIKE {}",
                        expr.item.display(input),
                        pattern.item.display(input)
                    )
                }
            }
            Expr::ILike { expr, pattern, not } => {
                if *not {
                    format!(
                        "{} NOT ILIKE {}",
                        expr.item.display(input),
                        pattern.item.display(input)
                    )
                } else {
                    format!(
                        "{} ILIKE {}",
                        expr.item.display(input),
                        pattern.item.display(input)
                    )
                }
            }
            Expr::Similar {
                expr,
                pattern,
                escape,
            } => {
                let mut s = format!(
                    "{} SIMILAR TO {}",
                    expr.item.display(input),
                    pattern.item.display(input)
                );
                if let Some(esc) = escape {
                    s.push_str(&format!(" ESCAPE {}", esc.item.display(input)));
                }
                s
            }
            Expr::FunctionCall {
                name,
                distinct,
                args,
                filter,
            } => {
                let distinct_str = if *distinct { "DISTINCT " } else { "" };
                let args_str = args
                    .items
                    .iter()
                    .map(|arg| arg.item.display(input))
                    .collect::<Vec<_>>()
                    .join(",");
                let mut s = format!("{}({}{})", name.item.display(input), distinct_str, args_str);
                if let Some(f) = filter {
                    s.push_str(&format!(" FILTER (WHERE {})", f.item.display(input)));
                }
                s
            }
            Expr::Array(items) => {
                let inner = items
                    .items
                    .iter()
                    .map(|e| e.item.display(input))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("ARRAY[{}]", inner)
            }
            Expr::Quantified { quantifier, expr } => {
                let q = match quantifier {
                    Quantifier::Any => "ANY",
                    Quantifier::Some => "SOME",
                    Quantifier::All => "ALL",
                };
                format!("{}({})", q, expr.item.display(input))
            }
            Expr::Subquery(query) => {
                format!("({})", query.item.display(input))
            }
            Expr::Case {
                operand,
                when_clauses,
                else_clause,
            } => {
                let mut result = String::from("CASE");

                if let Some(operand) = operand {
                    result.push_str(&format!(" {}", operand.item.display(input)));
                }

                for clause in when_clauses {
                    result.push_str(&format!(
                        " WHEN {} THEN {}",
                        clause.when.item.display(input),
                        clause.then.item.display(input)
                    ));
                }

                if let Some(else_expr) = else_clause {
                    result.push_str(&format!(" ELSE {}", else_expr.item.display(input)));
                }

                result.push_str(" END");
                result
            }
            Expr::In { expr, list, not } => {
                let mut result = expr.item.display(input);
                if *not {
                    result.push_str(" NOT IN (");
                } else {
                    result.push_str(" IN (");
                }
                match list {
                    InList::Subquery(query) => {
                        result.push_str(&query.item.display(input));
                    }
                    InList::Exprs(exprs) => {
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
            Expr::WindowFunction {
                name,
                args,
                over,
                filter,
            } => {
                let args_str = args
                    .items
                    .iter()
                    .map(|arg| arg.item.display(input))
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut result = format!("{}({})", name.item.display(input), args_str);
                if let Some(f) = filter {
                    result.push_str(&format!(" FILTER (WHERE {})", f.item.display(input)));
                }
                result.push_str(" OVER ");

                match over {
                    WindowOver::Name(w) => {
                        result.push_str(w.span.as_str(input));
                        return result;
                    }
                    WindowOver::Spec(spec) => {
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
            Expr::Empty => String::new(),
        }
    }
}

impl Display for Literal {
    fn display(&self, input: &str) -> String {
        match self {
            Literal::Number(number) => number.to_string(),
            Literal::Float(float) => float.to_string(),
            Literal::String(string) => {
                // The span includes the quotes, so just return as-is
                string.as_str(input).to_string()
            }
            Literal::Null => "NULL".to_string(),
            Literal::Boolean(b) => match b {
                Boolean::True => "TRUE".to_string(),
                Boolean::False => "FALSE".to_string(),
                Boolean::Unknown => "UNKNOWN".to_string(),
            },
            Literal::TypedString { data_type, value } => {
                let type_str = match data_type {
                    TypedLiteralKind::Date => "DATE",
                    TypedLiteralKind::Time => "TIME",
                    TypedLiteralKind::Timestamp => "TIMESTAMP",
                };
                format!("{} {}", type_str, value.as_str(input))
            }
        }
    }
}

impl Display for QualifiedName {
    fn display(&self, input: &str) -> String {
        self.parts
            .items
            .iter()
            .map(|part| part.item.display(input))
            .collect::<Vec<_>>()
            .join(".")
    }
}

impl Display for NamePart {
    fn display(&self, input: &str) -> String {
        match self {
            NamePart::Ident(ident) => ident.as_str(input).to_string(),
            NamePart::Star => "*".to_string(),
        }
    }
}

impl Display for DelimitedList<Node<SelectItem>> {
    fn display(&self, input: &str) -> String {
        self.items
            .iter()
            .map(|item| item.item.display(input))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Display for WindowFrame {
    fn display(&self, input: &str) -> String {
        let unit_str = match self.unit {
            FrameUnit::Rows => "ROWS",
            FrameUnit::Range => "RANGE",
            FrameUnit::Groups => "GROUPS",
        };
        let mut result = format!("{} BETWEEN {}", unit_str, self.start.item.display(input));
        if let Some(end) = &self.end {
            result.push_str(&format!(" AND {}", end.item.display(input)));
        }
        result
    }
}

impl Display for WindowSpec {
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

impl Display for WindowClause {
    fn display(&self, input: &str) -> String {
        self.windows
            .iter()
            .map(|w| w.item.display(input))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Display for WindowDef {
    fn display(&self, input: &str) -> String {
        format!(
            "{} AS ({})",
            self.name.as_str(input),
            self.spec.item.display(input)
        )
    }
}

impl Display for FrameBound {
    fn display(&self, input: &str) -> String {
        match self {
            FrameBound::UnboundedPreceding => "UNBOUNDED PRECEDING".to_string(),
            FrameBound::UnboundedFollowing => "UNBOUNDED FOLLOWING".to_string(),
            FrameBound::CurrentRow => "CURRENT ROW".to_string(),
            FrameBound::Preceding(expr) => format!("{} PRECEDING", expr.item.display(input)),
            FrameBound::Following(expr) => format!("{} FOLLOWING", expr.item.display(input)),
        }
    }
}

impl Display for FromClause {
    fn display(&self, input: &str) -> String {
        self.sources
            .items
            .iter()
            .map(|table| table.item.display(input))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Display for TableRef {
    fn display(&self, input: &str) -> String {
        match self {
            TableRef::Factor(factor) => factor.item.display(input),
            TableRef::Join {
                left,
                kind,
                right,
                constraint,
            } => {
                let mut result = left.item.display(input);

                // Add join type
                let join_str = match kind.base {
                    JoinBase::Inner => "INNER JOIN",
                    JoinBase::Left => {
                        if kind.outer {
                            "LEFT OUTER JOIN"
                        } else {
                            "LEFT JOIN"
                        }
                    }
                    JoinBase::Right => {
                        if kind.outer {
                            "RIGHT OUTER JOIN"
                        } else {
                            "RIGHT JOIN"
                        }
                    }
                    JoinBase::Full => {
                        if kind.outer {
                            "FULL OUTER JOIN"
                        } else {
                            "FULL JOIN"
                        }
                    }
                    JoinBase::Cross => "CROSS JOIN",
                };

                result.push_str(&format!(" {} {}", join_str, right.item.display(input)));

                // Add join constraint
                if let Some(constraint) = constraint {
                    match &constraint.item {
                        JoinConstraint::On(expr) => {
                            result.push_str(&format!(" ON {}", expr.item.display(input)));
                        }
                        JoinConstraint::Using(cols) => {
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

impl Display for TableFactor {
    fn display(&self, input: &str) -> String {
        match self {
            TableFactor::Named {
                name,
                alias,
                lateral,
            } => {
                let mut name_str = String::new();
                if *lateral {
                    name_str.push_str("LATERAL ");
                }
                name_str.push_str(&name.item.display(input));
                if let Some(alias) = alias {
                    format!("{} {}", name_str, alias.span.as_str(input))
                } else {
                    name_str
                }
            }
            TableFactor::Function {
                name,
                args,
                alias,
                columns,
                lateral,
            } => {
                let mut result = String::new();
                if *lateral {
                    result.push_str("LATERAL ");
                }
                let args_str = args
                    .items
                    .iter()
                    .map(|a| a.item.display(input))
                    .collect::<Vec<_>>()
                    .join(",");
                result.push_str(&format!("{}({})", name.item.display(input), args_str));
                if let Some(alias) = alias {
                    result.push_str(&format!(" AS {}", alias.span.as_str(input)));
                    if let Some(cols) = columns {
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
            TableFactor::Subquery { query, alias, .. } => {
                let mut result = String::new();
                // NOTE: LATERAL is not shown here; add if needed
                result.push_str(&format!("({})", query.item.display(input)));
                if let Some(alias) = alias {
                    result.push_str(&format!(" {}", alias.span.as_str(input)));
                }
                result
            }
            TableFactor::Parenthesized { inner } => {
                format!("({})", inner.item.display(input))
            }
        }
    }
}

impl Display for Query {
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

impl Display for With {
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

impl Display for CTE {
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
                Materialized::Materialized => result.push_str(" MATERIALIZED"),
                Materialized::NotMaterialized => result.push_str(" NOT MATERIALIZED"),
            }
        }
        result.push_str(&format!(" AS ({})", self.query.item.display(input)));
        result
    }
}

impl Display for QueryTail {
    fn display(&self, input: &str) -> String {
        let mut result = String::new();
        if let Some(order_by) = &self.order_by {
            result.push_str(&format!(" ORDER BY {}", order_by.item.display(input)));
        }
        match &self.limit {
            Some(l) if matches!(l.item.style, LimitStyle::Limit) => {
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

impl Display for OrderByClause {
    fn display(&self, input: &str) -> String {
        self.items
            .items
            .iter()
            .map(|item| item.item.display(input))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Display for OrderByItem {
    fn display(&self, input: &str) -> String {
        let mut result = self.expr.item.display(input);
        if let Some(direction) = &self.direction {
            match direction {
                OrderDirection::Asc => result.push_str(" ASC"),
                OrderDirection::Desc => result.push_str(" DESC"),
            }
        }
        result
    }
}

impl Display for LimitClause {
    fn display(&self, input: &str) -> String {
        match self.style {
            LimitStyle::FetchFirst => {
                format!("FETCH FIRST {} ROWS ONLY", self.count.item.display(input))
            }
            LimitStyle::Limit => format!("LIMIT {}", self.count.item.display(input)),
        }
    }
}

impl Display for OffsetClause {
    fn display(&self, input: &str) -> String {
        if self.rows_keyword {
            format!("OFFSET {} ROWS", self.count.item.display(input))
        } else {
            format!("OFFSET {}", self.count.item.display(input))
        }
    }
}

impl Display for QueryExpr {
    fn display(&self, input: &str) -> String {
        let mut result = self.left.item.display(input);

        for set_op_chain in &self.set_ops {
            let op_str = match set_op_chain.op {
                SetOp::Union { all } => {
                    if all {
                        " UNION ALL "
                    } else {
                        " UNION "
                    }
                }
                SetOp::Intersect { all } => {
                    if all {
                        " INTERSECT ALL "
                    } else {
                        " INTERSECT "
                    }
                }
                SetOp::Except { all } => {
                    if all {
                        " EXCEPT ALL "
                    } else {
                        " EXCEPT "
                    }
                }
                SetOp::Minus { all } => {
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

impl Display for QueryCore {
    fn display(&self, input: &str) -> String {
        match self {
            QueryCore::Select(stmt) => stmt.display(input),
            QueryCore::Values(_) => String::new(),
            QueryCore::Parenthesized(_) => String::new(),
        }
    }
}

impl Display for GroupByClause {
    fn display(&self, input: &str) -> String {
        self.items
            .items
            .iter()
            .map(|item| item.item.display(input))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Display for GroupByItem {
    fn display(&self, input: &str) -> String {
        match self {
            GroupByItem::Expr(expr) => expr.item.display(input),
            GroupByItem::Rollup(_) => String::new(),
            GroupByItem::Cube(_) => String::new(),
            GroupByItem::GroupingSets(_) => String::new(),
        }
    }
}
