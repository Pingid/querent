//! Concise SQL parser built on the `winnow` parser-combinator library.
//!
//! This is an experimental alternative to [`crate::parse`], implemented with
//! `winnow`'s [`TokenSlice`] stream and combinators (`opt`, `alt`, `repeat`,
//! `delimited`, ...). It parses the same [`crate::ast`] and is error-tolerant:
//! incomplete input yields a partial tree rather than a hard failure.
//!
//! Layering (top-down): public entry -> statement/query grammar -> expression
//! Pratt parser -> clause helpers -> atoms & combinators.
#![allow(dead_code)]

use winnow::Parser;
use winnow::combinator::delimited;
use winnow::combinator::fail;
use winnow::combinator::opt;
use winnow::combinator::repeat;
use winnow::error::EmptyError;
use winnow::error::ErrMode;
use winnow::stream::Stateful;
use winnow::stream::Stream;
use winnow::stream::TokenSlice;
use winnow::token::any;

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

/// Parser input: a stream over a borrowed token slice, carrying the active
/// [`DialectSpec`] as winnow state so grammar rules can consult dialect facts
/// (e.g. which keywords are reserved against bare-identifier use).
pub type Tokens<'a> = Stateful<TokenSlice<'a, Token<'a>>, &'a DialectSpec>;

/// Inner error carried by every parser. `EmptyError` is a zero-sized error: we
/// never inspect failure contents (parsing is tolerant), so this avoids the
/// per-backtrack allocation/setup of `ContextError`.
type PErr = ErrMode<EmptyError>;
/// Parser result alias.
type P<O> = Result<O, PErr>;

// ============================================================================
// Public API
// ============================================================================

/// Parse a single statement from `tokens`, tolerating incomplete input.
pub fn parse_statement<'a>(
    tokens: &'a [Token<'a>], spec: &'a DialectSpec,
) -> Option<Loc<Statement>> {
    let mut i = Tokens {
        input: TokenSlice::new(tokens),
        state: spec,
    };
    statement(&mut i).ok()
}

// ============================================================================
// Statements & queries
// ============================================================================

fn statement(i: &mut Tokens) -> P<Loc<Statement>> {
    while opt(tk(TK::Semicolon)).parse_next(i)?.is_some() {}
    let start = cur_start(i);
    match peek_kind(i) {
        None | Some(TK::Eof) => {
            let sp = i.first().map(|t| t.span).unwrap_or(Span::from(start));
            return Ok(Loc::new(start, Statement::Partial(Loc::new(sp, ()))));
        }
        Some(TK::Identifier) if matches!(peek_nth(i, 1), Some(TK::Eof) | None) => {
            let sp = i.first().map(|t| t.span).unwrap_or(Span::from(start));
            return Ok(Loc::new(start, Statement::Partial(Loc::new(sp, ()))));
        }
        Some(TK::Keyword(KW::Insert)) => return dml(i, start, parse_insert, Statement::Insert),
        Some(TK::Keyword(KW::Update)) => return dml(i, start, parse_update, Statement::Update),
        Some(TK::Keyword(KW::Delete)) => return dml(i, start, parse_delete, Statement::Delete),
        _ => {}
    }
    let q = spanned(query).parse_next(i)?;
    Ok(Loc::new((start, q.span.end), Statement::Query(q)))
}

/// Parse a spanned DML node and wrap it into a `Statement`.
fn dml<'a, T>(
    i: &mut Tokens<'a>, start: usize, body: fn(&mut Tokens<'a>) -> P<T>,
    wrap: fn(Loc<T>) -> Statement,
) -> P<Loc<Statement>> {
    let node = spanned(body).parse_next(i)?;
    Ok(Loc::new((start, node.span.end), wrap(node)))
}

fn query(i: &mut Tokens) -> P<Query> {
    Ok(Query {
        with: opt(spanned(parse_with)).parse_next(i)?,
        body: opt(spanned(query_expr)).parse_next(i)?,
        tail: opt(spanned(query_suffix)).parse_next(i)?,
    })
}

fn parse_with(i: &mut Tokens) -> P<With> {
    kw(KW::With).parse_next(i)?;
    let recursive = opt(kw(KW::Recursive)).parse_next(i)?.is_some();
    let mut ctes = Vec::new();
    // Zero CTEs is tolerated (partial input such as `WITH ^`).
    loop {
        let start = cur_start(i);
        let Some(name) = opt(ident).parse_next(i)? else {
            break;
        };
        let columns = opt(parens(comma_list(ident))).parse_next(i)?;
        let materialized = if opt(kw(KW::Materialized)).parse_next(i)?.is_some() {
            Some(CteMaterialization::Materialized)
        } else if opt((op(OpTag::Not), kw(KW::Materialized)))
            .parse_next(i)?
            .is_some()
        {
            Some(CteMaterialization::NotMaterialized)
        } else {
            None
        };
        kw(KW::As).parse_next(i)?;
        let query = Box::new(parens(spanned(query)).parse_next(i)?);
        ctes.push(Loc::new(
            (start, end_from(i, start)),
            Cte {
                name,
                columns,
                materialized,
                query,
            },
        ));
        if opt(tk(TK::Comma)).parse_next(i)?.is_none() {
            break;
        }
    }
    Ok(With { recursive, ctes })
}

fn query_expr(i: &mut Tokens) -> P<QueryExpr> {
    let left = spanned(query_primary).parse_next(i)?;
    let set_ops: Vec<Loc<SetOpTerm>> = repeat(0.., spanned(set_op_term)).parse_next(i)?;
    Ok(QueryExpr { left, set_ops })
}

fn set_op_term(i: &mut Tokens) -> P<SetOpTerm> {
    let op = set_op(i)?;
    Ok(SetOpTerm {
        op,
        right: spanned(query_primary).parse_next(i)?,
    })
}

fn set_op(i: &mut Tokens) -> P<SetOp> {
    let which = match peek_kind(i) {
        Some(TK::Keyword(w @ (KW::Union | KW::Intersect | KW::Except | KW::Minus))) => w,
        _ => return fail(i),
    };
    any.parse_next(i)?;
    let all = opt(kw(KW::All)).parse_next(i)?.is_some();
    Ok(match which {
        KW::Union => SetOp::Union { all },
        KW::Intersect => SetOp::Intersect { all },
        KW::Except => SetOp::Except { all },
        _ => SetOp::Minus { all },
    })
}

fn query_primary(i: &mut Tokens) -> P<QueryPrimary> {
    match peek_kind(i) {
        Some(TK::Keyword(KW::Select)) => {
            Ok(QueryPrimary::Select(spanned(parse_select).parse_next(i)?))
        }
        Some(TK::Keyword(KW::Values)) => {
            Ok(QueryPrimary::Values(spanned(parse_values).parse_next(i)?))
        }
        _ => fail(i),
    }
}

// ============================================================================
// SELECT & DML
// ============================================================================

fn parse_select(i: &mut Tokens) -> P<Select> {
    kw(KW::Select).parse_next(i)?;
    let distinct = opt(parse_distinct)
        .parse_next(i)?
        .unwrap_or(SetQuantifier::All);
    // Projection span spans the gap from the keyword to the next token, so the
    // cursor in `SELECT ^ FROM` lands inside an (empty) projection.
    let proj_start = end_from(i, cur_start(i));
    let proj = opt(parse_projection).parse_next(i)?.unwrap_or_default();
    let projection = Loc::new((proj_start, cur_start(i)), proj);
    Ok(Select {
        distinct,
        projection,
        from: opt(spanned(parse_from)).parse_next(i)?,
        where_clause: opt(spanned(parse_where)).parse_next(i)?,
        group_by: opt(spanned(parse_group_by)).parse_next(i)?,
        having: opt(parse_having).parse_next(i)?,
        window: opt(spanned(parse_window)).parse_next(i)?,
        qualify: None,
    })
}

fn parse_distinct(i: &mut Tokens) -> P<SetQuantifier> {
    kw(KW::Distinct).parse_next(i)?;
    if opt(kw(KW::On)).parse_next(i)?.is_some() {
        return Ok(SetQuantifier::DistinctOn(parens(args).parse_next(i)?));
    }
    Ok(SetQuantifier::Distinct)
}

fn parse_projection(i: &mut Tokens) -> P<Projection> {
    Ok(Projection {
        list: opt(comma_list(projection_item))
            .parse_next(i)?
            .unwrap_or_default(),
    })
}

fn projection_item(i: &mut Tokens) -> P<ProjectionItem> {
    Ok(ProjectionItem {
        expr: spanned(expr_or_empty).parse_next(i)?,
        alias: alias(i)?,
    })
}

fn parse_insert(i: &mut Tokens) -> P<Insert> {
    kw(KW::Insert).parse_next(i)?;
    kw(KW::Into).parse_next(i)?;
    let table = spanned(qualified_name).parse_next(i)?;
    let columns = opt(parens(comma_list(ident))).parse_next(i)?;
    let source = if opt(kw(KW::Default)).parse_next(i)?.is_some() {
        opt(kw(KW::Values)).parse_next(i)?;
        InsertSource::Default
    } else if peek_kind(i) == Some(TK::Keyword(KW::Values)) {
        InsertSource::Values(spanned(parse_values).parse_next(i)?)
    } else {
        InsertSource::Query(Box::new(spanned(query).parse_next(i)?))
    };
    Ok(Insert {
        table,
        columns,
        source,
        returning: opt(parse_returning).parse_next(i)?,
    })
}

fn parse_values(i: &mut Tokens) -> P<Values> {
    kw(KW::Values).parse_next(i)?;
    let mut rows = Vec::new();
    loop {
        let start = cur_start(i);
        let row = parens(args).parse_next(i)?;
        rows.push(Loc::new((start, end_from(i, start)), row));
        if opt(tk(TK::Comma)).parse_next(i)?.is_none() {
            break;
        }
    }
    Ok(Values { rows })
}

fn parse_update(i: &mut Tokens) -> P<Update> {
    kw(KW::Update).parse_next(i)?;
    let table = spanned(qualified_name).parse_next(i)?;
    let alias = alias(i)?;
    kw(KW::Set).parse_next(i)?;
    let assignments = comma_list(parse_assignment).parse_next(i)?;
    Ok(Update {
        table,
        alias,
        assignments,
        from: opt(spanned(parse_from)).parse_next(i)?,
        where_clause: opt(spanned(parse_where)).parse_next(i)?,
        returning: opt(parse_returning).parse_next(i)?,
    })
}

fn parse_assignment(i: &mut Tokens) -> P<Assignment> {
    let column = spanned(ident).parse_next(i)?;
    op(OpTag::Eq).parse_next(i)?;
    Ok(Assignment {
        column,
        value: spanned(expr).parse_next(i)?,
    })
}

fn parse_delete(i: &mut Tokens) -> P<Delete> {
    kw(KW::Delete).parse_next(i)?;
    kw(KW::From).parse_next(i)?;
    let table = spanned(qualified_name).parse_next(i)?;
    let alias = alias(i)?;
    let using = if opt(kw(KW::Using)).parse_next(i)?.is_some() {
        let start = cur_start(i);
        let sources = opt(comma_list(table_ref))
            .parse_next(i)?
            .unwrap_or_default();
        Some(Loc::new((start, end_from(i, start)), From { sources }))
    } else {
        None
    };
    Ok(Delete {
        table,
        alias,
        using,
        where_clause: opt(spanned(parse_where)).parse_next(i)?,
        returning: opt(parse_returning).parse_next(i)?,
    })
}

fn parse_returning(i: &mut Tokens) -> P<Loc<Projection>> {
    kw(KW::Returning).parse_next(i)?;
    spanned(parse_projection).parse_next(i)
}

// ============================================================================
// FROM / JOIN
// ============================================================================

fn parse_from(i: &mut Tokens) -> P<From> {
    kw(KW::From).parse_next(i)?;
    // A partial source list (e.g. a dangling `JOIN`) is swallowed *without*
    // rewinding: consumed tokens still extend the FROM span up to the cursor.
    let sources = comma_list(table_ref).parse_next(i).unwrap_or_default();
    Ok(From { sources })
}

fn table_ref(i: &mut Tokens) -> P<TableRef> {
    let start = cur_start(i);
    let mut left = table_factor(i)?;
    while let Some(kind) = opt(parse_join_kind).parse_next(i)? {
        let rs = cur_start(i);
        // A missing right factor (e.g. `a JOIN ^`) fails the whole ref, but the
        // join keyword stays consumed (no rewind), so `parse_from` still spans
        // up to the cursor for completion.
        let right = table_factor(i)?;
        let constraint = opt(spanned(parse_join_constraint)).parse_next(i)?;
        let span = Span::new(start, end_from(i, rs));
        let right_span = Span::new(rs, span.end);
        left = TableRef::Join(Loc::new(
            span,
            Join {
                left: Box::new(Loc::new(span, left)),
                kind,
                right: Box::new(Loc::new(right_span, right)),
                constraint,
            },
        ));
    }
    Ok(left)
}

fn table_factor(i: &mut Tokens) -> P<TableRef> {
    let start = cur_start(i);
    let lateral = opt(kw(KW::Lateral)).parse_next(i)?.is_some();

    if opt(tk(TK::LeftParen)).parse_next(i)?.is_some() {
        if is_query_start(i) {
            let q = spanned(query).parse_next(i)?;
            opt(tk(TK::RightParen)).parse_next(i)?;
            let alias = alias(i)?;
            let span = Span::new(start, end_from(i, start));
            let factor = TableFactor::Subquery(Loc::new(
                span,
                SubqueryTableFactor {
                    query: q,
                    alias,
                    lateral,
                },
            ));
            return Ok(TableRef::Factor(Loc::new(span, factor)));
        }
        let inner = Box::new(spanned(table_ref).parse_next(i)?);
        opt(tk(TK::RightParen)).parse_next(i)?;
        let span = Span::new(start, end_from(i, start));
        return Ok(TableRef::Factor(Loc::new(
            span,
            TableFactor::Parenthesized(inner),
        )));
    }

    let name = spanned(qualified_name).parse_next(i)?;
    if opt(tk(TK::LeftParen)).parse_next(i)?.is_some() {
        let func_args = args(i)?;
        opt(tk(TK::RightParen)).parse_next(i)?;
        let alias = alias(i)?;
        let columns = opt(parens(comma_list(ident_or_keyword))).parse_next(i)?;
        let span = Span::new(start, end_from(i, start));
        let factor = TableFactor::Function(Loc::new(
            span,
            FunctionTableFactor {
                name,
                args: func_args,
                alias,
                columns,
                lateral,
            },
        ));
        return Ok(TableRef::Factor(Loc::new(span, factor)));
    }

    let alias = alias(i)?;
    let span = Span::new(start, end_from(i, start));
    let factor = TableFactor::Named(Loc::new(
        span,
        NamedTableFactor {
            name,
            alias,
            lateral,
        },
    ));
    Ok(TableRef::Factor(Loc::new(span, factor)))
}

fn parse_join_kind(i: &mut Tokens) -> P<JoinKind> {
    let natural = opt(kw(KW::Natural)).parse_next(i)?.is_some();
    let (base, outer) = match peek_kind(i) {
        Some(TK::Keyword(KW::Inner)) => {
            any.parse_next(i)?;
            kw(KW::Join).parse_next(i)?;
            (JoinBase::Inner, false)
        }
        Some(TK::Keyword(w @ (KW::Left | KW::Right | KW::Full))) => {
            any.parse_next(i)?;
            let outer = opt(kw(KW::Outer)).parse_next(i)?.is_some();
            kw(KW::Join).parse_next(i)?;
            let base = match w {
                KW::Left => JoinBase::Left,
                KW::Right => JoinBase::Right,
                _ => JoinBase::Full,
            };
            (base, outer)
        }
        Some(TK::Keyword(KW::Cross)) => {
            any.parse_next(i)?;
            kw(KW::Join).parse_next(i)?;
            (JoinBase::Cross, false)
        }
        Some(TK::Keyword(KW::Join)) => {
            any.parse_next(i)?;
            (JoinBase::Inner, false)
        }
        _ => return fail(i),
    };
    Ok(JoinKind {
        base,
        outer,
        natural,
    })
}

fn parse_join_constraint(i: &mut Tokens) -> P<JoinConstraint> {
    if opt(kw(KW::On)).parse_next(i)?.is_some() {
        return Ok(JoinConstraint::On(spanned(expr).parse_next(i)?));
    }
    if opt(kw(KW::Using)).parse_next(i)?.is_some() {
        return Ok(JoinConstraint::Using(
            parens(comma_list(ident)).parse_next(i)?,
        ));
    }
    fail(i)
}

// ============================================================================
// WHERE / GROUP BY / HAVING / WINDOW
// ============================================================================

fn parse_where(i: &mut Tokens) -> P<Where> {
    kw(KW::Where).parse_next(i)?;
    Ok(Where {
        expr: spanned(expr_or_empty).parse_next(i)?,
    })
}

fn parse_group_by(i: &mut Tokens) -> P<GroupBy> {
    (kw(KW::Group), kw(KW::By)).parse_next(i)?;
    Ok(GroupBy {
        items: opt(comma_list(group_by_item))
            .parse_next(i)?
            .unwrap_or_default(),
    })
}

fn group_by_item(i: &mut Tokens) -> P<GroupByItem> {
    if opt(kw(KW::Rollup)).parse_next(i)?.is_some() {
        return Ok(GroupByItem::Rollup(
            parens(comma_list(expr)).parse_next(i)?.items,
        ));
    }
    if opt(kw(KW::Cube)).parse_next(i)?.is_some() {
        return Ok(GroupByItem::Cube(
            parens(comma_list(expr)).parse_next(i)?.items,
        ));
    }
    if opt((kw(KW::Grouping), kw(KW::Sets)))
        .parse_next(i)?
        .is_some()
    {
        return Ok(GroupByItem::GroupingSets(
            parens(comma_list(grouping_set)).parse_next(i)?.items,
        ));
    }
    Ok(GroupByItem::Expr(spanned(expr).parse_next(i)?))
}

fn grouping_set(i: &mut Tokens) -> P<GroupingSet> {
    if try_with(i, |i| (tk(TK::LeftParen), tk(TK::RightParen)).parse_next(i)).is_some() {
        return Ok(GroupingSet::Exprs(Vec::new()));
    }
    if let Some(list) = try_with(i, |i| parens(comma_list(expr)).parse_next(i)) {
        return Ok(GroupingSet::Exprs(list.items));
    }
    Ok(GroupingSet::Expr(spanned(expr).parse_next(i)?))
}

fn parse_having(i: &mut Tokens) -> P<Loc<Expr>> {
    kw(KW::Having).parse_next(i)?;
    spanned(expr_or_empty).parse_next(i)
}

fn parse_window(i: &mut Tokens) -> P<Window> {
    kw(KW::Window).parse_next(i)?;
    let mut windows = Vec::new();
    loop {
        let name = ident(i)?;
        kw(KW::As).parse_next(i)?;
        let spec = parens(spanned(parse_window_spec)).parse_next(i)?;
        windows.push(Loc::new(name, WindowDef { name, spec }));
        if opt(tk(TK::Comma)).parse_next(i)?.is_none() {
            break;
        }
    }
    Ok(Window { windows })
}

fn parse_window_ref(i: &mut Tokens) -> P<WindowRef> {
    if let Some(spec) = try_with(i, |i| parens(spanned(parse_window_spec)).parse_next(i)) {
        return Ok(WindowRef::Spec(Box::new(spec)));
    }
    Ok(WindowRef::Name(spanned(ident).parse_next(i)?))
}

fn parse_window_spec(i: &mut Tokens) -> P<WindowSpec> {
    let partition_by = if opt((kw(KW::Partition), kw(KW::By)))
        .parse_next(i)?
        .is_some()
    {
        opt(comma_list(expr)).parse_next(i)?
    } else {
        None
    };
    Ok(WindowSpec {
        partition_by,
        order_by: opt(spanned(parse_order_by)).parse_next(i)?,
        frame: opt(spanned(parse_window_frame))
            .parse_next(i)?
            .map(Box::new),
    })
}

fn parse_window_frame(i: &mut Tokens) -> P<WindowFrame> {
    let unit = match peek_kind(i) {
        Some(TK::Keyword(KW::Rows)) => FrameUnit::Rows,
        Some(TK::Keyword(KW::Range)) => FrameUnit::Range,
        _ => return fail(i),
    };
    any.parse_next(i)?;
    op(OpTag::Between).parse_next(i)?;
    let start = Box::new(spanned(parse_frame_bound).parse_next(i)?);
    opt(op(OpTag::And)).parse_next(i)?;
    let end = Some(Box::new(spanned(parse_frame_bound).parse_next(i)?));
    Ok(WindowFrame { unit, start, end })
}

fn parse_frame_bound(i: &mut Tokens) -> P<FrameBound> {
    match peek_kind(i) {
        Some(TK::Keyword(KW::Unbounded)) => {
            any.parse_next(i)?;
            if opt(kw(KW::Preceding)).parse_next(i)?.is_some() {
                Ok(FrameBound::UnboundedPreceding)
            } else {
                kw(KW::Following).parse_next(i)?;
                Ok(FrameBound::UnboundedFollowing)
            }
        }
        Some(TK::Keyword(KW::Current)) => {
            any.parse_next(i)?;
            kw(KW::Row).parse_next(i)?;
            Ok(FrameBound::CurrentRow)
        }
        _ => {
            let e = Box::new(spanned(expr).parse_next(i)?);
            if opt(kw(KW::Preceding)).parse_next(i)?.is_some() {
                Ok(FrameBound::Preceding(e))
            } else {
                kw(KW::Following).parse_next(i)?;
                Ok(FrameBound::Following(e))
            }
        }
    }
}

// ============================================================================
// ORDER BY / LIMIT / OFFSET
// ============================================================================

fn query_suffix(i: &mut Tokens) -> P<QuerySuffix> {
    let (mut order_by, mut limit, mut offset) = (None, None, None);
    loop {
        let mut progressed = false;
        if order_by.is_none() {
            order_by = opt(spanned(parse_order_by)).parse_next(i)?;
            progressed = order_by.is_some();
        }
        if !progressed && limit.is_none() {
            limit = opt(spanned(parse_limit)).parse_next(i)?;
            progressed = limit.is_some();
        }
        if !progressed && offset.is_none() {
            offset = opt(spanned(parse_offset)).parse_next(i)?;
            progressed = offset.is_some();
        }
        if !progressed {
            break;
        }
    }
    if order_by.is_some() || limit.is_some() || offset.is_some() {
        Ok(QuerySuffix {
            order_by,
            limit,
            offset,
        })
    } else {
        fail(i)
    }
}

fn parse_order_by(i: &mut Tokens) -> P<OrderBy> {
    (kw(KW::Order), kw(KW::By)).parse_next(i)?;
    Ok(OrderBy {
        items: comma_list(order_by_item).parse_next(i)?,
    })
}

fn order_by_item(i: &mut Tokens) -> P<OrderByItem> {
    let expr = spanned(expr).parse_next(i)?;
    let direction = match peek_kind(i) {
        Some(TK::Keyword(KW::Asc)) => Some(SortDirection::Asc),
        Some(TK::Keyword(KW::Desc)) => Some(SortDirection::Desc),
        _ => None,
    };
    if direction.is_some() {
        any.parse_next(i)?;
    }
    let nulls = if opt(kw(KW::Nulls)).parse_next(i)?.is_some() {
        match peek_kind(i) {
            Some(TK::Keyword(KW::First)) => {
                any.parse_next(i)?;
                Some(NullsOrder::First)
            }
            Some(TK::Keyword(KW::Last)) => {
                any.parse_next(i)?;
                Some(NullsOrder::Last)
            }
            _ => None,
        }
    } else {
        None
    };
    Ok(OrderByItem {
        expr,
        direction,
        nulls,
    })
}

fn parse_limit(i: &mut Tokens) -> P<Limit> {
    if opt((kw(KW::Fetch), kw(KW::First))).parse_next(i)?.is_some() {
        let count = spanned(expr).parse_next(i)?;
        opt(kw(KW::Rows)).parse_next(i)?;
        opt(kw(KW::Only)).parse_next(i)?;
        return Ok(Limit {
            count,
            style: LimitKind::FetchFirst,
        });
    }
    kw(KW::Limit).parse_next(i)?;
    Ok(Limit {
        count: spanned(expr).parse_next(i)?,
        style: LimitKind::Limit,
    })
}

fn parse_offset(i: &mut Tokens) -> P<Offset> {
    kw(KW::Offset).parse_next(i)?;
    Ok(Offset {
        count: spanned(expr).parse_next(i)?,
        rows_keyword: opt(kw(KW::Rows)).parse_next(i)?.is_some(),
    })
}

// ============================================================================
// Expressions (Pratt / precedence climbing)
// ============================================================================

fn expr(i: &mut Tokens) -> P<Expr> {
    expr_bp(i, 0)
}

/// Tolerant expression: yields `Expr::Empty` on failure *without* rewinding, so
/// any tokens consumed by a partial expression still extend the enclosing span
/// up to the cursor (needed by the completion engine for `SELECT CASE WHEN ^`).
fn expr_or_empty(i: &mut Tokens) -> P<Expr> {
    Ok(expr(i).unwrap_or(Expr::Empty))
}

fn expr_bp(i: &mut Tokens, min_bp: u8) -> P<Expr> {
    let start = cur_start(i);
    let mut lhs = spanned(prefix_expr).parse_next(i)?;
    while let Some((tag, prec, r_bp)) = peek_infix(i) {
        if prec < min_bp {
            break;
        }
        lhs = infix(i, lhs, start, tag, r_bp)?;
    }
    Ok(lhs.item)
}

/// Parse a sub-expression with binding power, capturing its span.
fn spanned_bp(i: &mut Tokens, bp: u8) -> P<Loc<Expr>> {
    let start = cur_start(i);
    let v = expr_bp(i, bp)?;
    Ok(Loc::new((start, end_from(i, start)), v))
}

/// Inspect (without consuming) the binding power of an upcoming infix operator.
fn peek_infix(i: &Tokens) -> Option<(OpTag, u8, u8)> {
    match i.first()?.kind {
        TK::Operator(o) if o.fixity == Fixity::Infix => {
            let r_bp = match o.assoc {
                Assoc::Right => o.precedence,
                _ => o.precedence + 1,
            };
            Some((o.semantic_tag, o.precedence, r_bp))
        }
        // NOT acts as infix only when followed by BETWEEN/LIKE/ILIKE/SIMILAR/IN.
        TK::Operator(o) if o.semantic_tag == OpTag::Not => match peek_nth(i, 1) {
            Some(TK::Operator(n))
                if matches!(
                    n.semantic_tag,
                    OpTag::Between | OpTag::Like | OpTag::Ilike | OpTag::Similar | OpTag::In
                ) =>
            {
                Some((OpTag::Not, n.precedence, n.precedence + 1))
            }
            _ => None,
        },
        _ => None,
    }
}

fn infix(i: &mut Tokens, left: Loc<Expr>, start: usize, tag: OpTag, r_bp: u8) -> P<Loc<Expr>> {
    let op_span = any.parse_next(i)?.span;
    match tag {
        OpTag::Not => infix_not(i, left, start),
        OpTag::Between => infix_between(i, left, start, false),
        OpTag::Like | OpTag::Ilike => infix_like(i, left, start, tag, false),
        OpTag::Similar => infix_similar(i, left, start),
        OpTag::In => infix_in(i, left, start, false),
        OpTag::Is => infix_is(i, left, start),
        OpTag::TypeCast => infix_cast(i, left, start),
        _ => {
            let right = try_with(i, |i| spanned_bp(i, r_bp));
            let end = right
                .as_ref()
                .map(|r| r.span.end)
                .unwrap_or_else(|| end_from(i, start));
            let binary = Binary {
                left: Box::new(left),
                op: Some(Loc::new(op_span, tag)),
                right: right.map(Box::new),
            };
            let bexpr = Expr::Binary(Loc::new((binary.left.span.start, end), binary));
            Ok(Loc::new((start, end), bexpr))
        }
    }
}

fn infix_not(i: &mut Tokens, left: Loc<Expr>, start: usize) -> P<Loc<Expr>> {
    match peek_kind(i) {
        Some(TK::Operator(o)) if o.semantic_tag == OpTag::Between => {
            any.parse_next(i)?;
            infix_between(i, left, start, true)
        }
        Some(TK::Operator(o)) if o.semantic_tag == OpTag::Like => {
            any.parse_next(i)?;
            infix_like(i, left, start, OpTag::Like, true)
        }
        Some(TK::Operator(o)) if o.semantic_tag == OpTag::Ilike => {
            any.parse_next(i)?;
            infix_like(i, left, start, OpTag::Ilike, true)
        }
        Some(TK::Operator(o)) if o.semantic_tag == OpTag::In => {
            any.parse_next(i)?;
            infix_in(i, left, start, true)
        }
        _ => {
            let e = Box::new(spanned_bp(i, 3)?);
            let span = (left.span.start, e.span.end);
            Ok(Loc::new(
                (start, e.span.end),
                Expr::Unary(Loc::new(
                    span,
                    Unary {
                        op_tok: Loc::new(start, OpTag::Not),
                        expr: e,
                    },
                )),
            ))
        }
    }
}

fn infix_between(i: &mut Tokens, left: Loc<Expr>, start: usize, not: bool) -> P<Loc<Expr>> {
    if let Some(low) = try_with(i, |i| spanned_bp(i, 10)) {
        if opt(op(OpTag::And)).parse_next(i)?.is_some()
            && let Some(high) = try_with(i, |i| spanned_bp(i, 10))
        {
            let span = (start, high.span.end);
            let between = Between {
                expr: Box::new(left),
                low: Box::new(low),
                high: Box::new(high),
                not,
            };
            return Ok(Loc::new(span, Expr::Between(Loc::new(span, between))));
        }
        // Only a low bound parsed: keep it as a partial binary.
        let span = (start, low.span.end);
        let binary = Binary {
            left: Box::new(left),
            op: Some(Loc::new(start, OpTag::Between)),
            right: Some(Box::new(low)),
        };
        let bspan = binary.left.span.join(span.into());
        return Ok(Loc::new(span, Expr::Binary(Loc::new(bspan, binary))));
    }
    // No bounds at all.
    let (l_start, l_end) = (left.span.start, left.span.end);
    let end = end_from(i, l_end);
    let binary = Binary {
        left: Box::new(left),
        op: Some(Loc::new(l_end, OpTag::Between)),
        right: None,
    };
    Ok(Loc::new(
        (start, end),
        Expr::Binary(Loc::new((l_start, end), binary)),
    ))
}

fn infix_like(
    i: &mut Tokens, left: Loc<Expr>, start: usize, tag: OpTag, not: bool,
) -> P<Loc<Expr>> {
    let pattern = spanned_bp(i, 10)?;
    let span = (start, pattern.span.end);
    let (expr, pattern) = (Box::new(left), Box::new(pattern));
    let e = match tag {
        OpTag::Like => Expr::Like(Loc::new(span, Like { expr, pattern, not })),
        _ => Expr::ILike(Loc::new(span, ILike { expr, pattern, not })),
    };
    Ok(Loc::new(span, e))
}

fn infix_similar(i: &mut Tokens, left: Loc<Expr>, start: usize) -> P<Loc<Expr>> {
    kw(KW::To).parse_next(i)?;
    let pattern = spanned_bp(i, 10)?;
    let escape = if opt(kw(KW::Escape)).parse_next(i)?.is_some() {
        Some(Box::new(spanned_bp(i, 10)?))
    } else {
        None
    };
    let end = escape
        .as_ref()
        .map(|e| e.span.end)
        .unwrap_or(pattern.span.end);
    let span = (start, end);
    let similar = Similar {
        expr: Box::new(left),
        pattern: Box::new(pattern),
        escape,
    };
    Ok(Loc::new(span, Expr::Similar(Loc::new(span, similar))))
}

fn infix_in(i: &mut Tokens, left: Loc<Expr>, start: usize, not: bool) -> P<Loc<Expr>> {
    tk(TK::LeftParen).parse_next(i)?;
    let list = if is_query_start(i) {
        ExprList::Subquery(Box::new(spanned(query).parse_next(i)?))
    } else {
        ExprList::Exprs(args(i)?.items)
    };
    opt(tk(TK::RightParen)).parse_next(i)?;
    let span = (start, end_from(i, start));
    Ok(Loc::new(
        span,
        Expr::In(Loc::new(
            span,
            In {
                expr: Box::new(left),
                list,
                not,
            },
        )),
    ))
}

fn infix_is(i: &mut Tokens, left: Loc<Expr>, start: usize) -> P<Loc<Expr>> {
    let not = opt(op(OpTag::Not)).parse_next(i)?.is_some();
    kw(KW::Null).parse_next(i)?;
    let span = (start, end_from(i, start));
    Ok(Loc::new(
        span,
        Expr::IsNull(Loc::new(
            span,
            IsNull {
                expr: Box::new(left),
                not,
            },
        )),
    ))
}

fn infix_cast(i: &mut Tokens, left: Loc<Expr>, start: usize) -> P<Loc<Expr>> {
    let data_type = spanned(parse_data_type).parse_next(i)?;
    let span = (start, end_from(i, start));
    Ok(Loc::new(
        span,
        Expr::Cast(Loc::new(
            span,
            Cast {
                expr: Box::new(left),
                data_type,
            },
        )),
    ))
}

fn prefix_expr(i: &mut Tokens) -> P<Expr> {
    if let Some(TK::Operator(o)) = peek_kind(i) {
        if o.fixity == Fixity::Prefix {
            if o.semantic_tag == OpTag::Exists {
                any.parse_next(i)?;
                return Ok(Expr::Exists(Box::new(
                    parens(spanned(query)).parse_next(i)?,
                )));
            }
            let op_span = any.parse_next(i)?.span;
            let e = Box::new(spanned_bp(i, o.precedence)?);
            let unary = Unary {
                op_tok: Loc::new(op_span, o.semantic_tag),
                expr: e,
            };
            return Ok(Expr::Unary(Loc::new(op_span.join(unary.expr.span), unary)));
        }
        if matches!(o.semantic_tag, OpTag::Add | OpTag::Sub) {
            let op_span = any.parse_next(i)?.span;
            let tag = if o.semantic_tag == OpTag::Add {
                OpTag::UnaryPlus
            } else {
                OpTag::UnaryMinus
            };
            let e = Box::new(spanned_bp(i, 8)?);
            let unary = Unary {
                op_tok: Loc::new(op_span, tag),
                expr: e,
            };
            return Ok(Expr::Unary(Loc::new(op_span.join(unary.expr.span), unary)));
        }
    }
    primary_expr(i)
}

fn primary_expr(i: &mut Tokens) -> P<Expr> {
    let start = cur_start(i);

    // Parenthesized expression or subquery.
    if let Some(open) = opt(tk(TK::LeftParen)).parse_next(i)? {
        if is_query_start(i) {
            let q = spanned(query).parse_next(i)?;
            opt(tk(TK::RightParen)).parse_next(i)?;
            return parse_postfix(i, start, Expr::Subquery(Box::new(q)));
        }
        let inner = Box::new(spanned(expr).parse_next(i)?);
        let close = opt(tk(TK::RightParen)).parse_next(i)?.map(|t| t.span);
        let end = close.map(|s| s.end).unwrap_or(inner.span.end);
        let paren = Paren {
            open: open.span,
            expr: inner,
            close,
        };
        return parse_postfix(i, start, Expr::Paren(Loc::new((start, end), paren)));
    }

    // Keyword-led primaries.
    if opt(kw(KW::Cast)).parse_next(i)?.is_some() {
        let e = parse_cast(i, start)?;
        return parse_postfix(i, start, e);
    }
    if opt(kw(KW::Case)).parse_next(i)?.is_some() {
        let e = parse_case(i, start)?;
        return parse_postfix(i, start, e);
    }
    if opt(kw(KW::Row)).parse_next(i)?.is_some() {
        let exprs = parens(args).parse_next(i)?;
        let e = Expr::Row(Loc::new((start, end_from(i, start)), Row { exprs }));
        return parse_postfix(i, start, e);
    }
    if opt(kw(KW::Array)).parse_next(i)?.is_some() {
        opt(tk(TK::LeftBracket)).parse_next(i)?;
        let items = args(i)?;
        opt(tk(TK::RightBracket)).parse_next(i)?;
        return parse_postfix(i, start, Expr::Array(items));
    }
    if let Some(q) = try_with(i, |i| parse_quantified(i, start)) {
        return parse_postfix(i, start, q);
    }
    if let Some(lit) = try_with(i, |i| parse_typed_literal(i, start)) {
        return parse_postfix(i, start, Expr::Literal(lit));
    }
    if let Some(lit) = opt(parse_literal).parse_next(i)? {
        return parse_postfix(i, start, Expr::Literal(lit));
    }

    // Column name or function call.
    let name = spanned(qualified_name).parse_next(i)?;
    if opt(tk(TK::LeftParen)).parse_next(i)?.is_some() {
        return parse_func_call(i, name, start);
    }
    parse_postfix(i, start, Expr::Name(name))
}

/// Postfix chain: array subscripts `e[i]` / `e[lo:hi]` and `AT TIME ZONE`.
fn parse_postfix(i: &mut Tokens, start: usize, mut e: Expr) -> P<Expr> {
    loop {
        if opt(tk(TK::LeftBracket)).parse_next(i)?.is_some() {
            let index = Box::new(spanned(expr).parse_next(i)?);
            let upper = if opt(tk(TK::Colon)).parse_next(i)?.is_some() {
                Some(Box::new(spanned(expr).parse_next(i)?))
            } else {
                None
            };
            opt(tk(TK::RightBracket)).parse_next(i)?;
            let span = Span::new(start, end_from(i, start));
            e = Expr::Subscript(Loc::new(
                span,
                Subscript {
                    expr: Box::new(Loc::new(span, e)),
                    index,
                    upper,
                },
            ));
            continue;
        }
        if opt(kw(KW::At)).parse_next(i)?.is_some() {
            kw(KW::Time).parse_next(i)?;
            kw(KW::Zone).parse_next(i)?;
            let timezone = Box::new(spanned(expr).parse_next(i)?);
            let span = Span::new(start, end_from(i, start));
            e = Expr::AtTimeZone(Loc::new(
                span,
                AtTimeZone {
                    expr: Box::new(Loc::new(span, e)),
                    timezone,
                },
            ));
            continue;
        }
        break;
    }
    Ok(e)
}

fn parse_cast(i: &mut Tokens, start: usize) -> P<Expr> {
    tk(TK::LeftParen).parse_next(i)?;
    let expr = Box::new(spanned(expr).parse_next(i)?);
    kw(KW::As).parse_next(i)?;
    let data_type = spanned(parse_data_type).parse_next(i)?;
    opt(tk(TK::RightParen)).parse_next(i)?;
    Ok(Expr::Cast(Loc::new(
        (start, end_from(i, start)),
        Cast { expr, data_type },
    )))
}

fn parse_data_type(i: &mut Tokens) -> P<DataType> {
    let name = spanned(qualified_name).parse_next(i)?;
    if opt(tk(TK::LeftParen)).parse_next(i)?.is_none() {
        return Ok(DataType::Named(name));
    }
    let mut params = Vec::new();
    loop {
        match peek_kind(i) {
            Some(TK::Number) => {
                let t = i.first().unwrap();
                let (sp, text) = (t.span, t.text);
                let Ok(n) = text.parse::<i64>() else { break };
                any.parse_next(i)?;
                params.push(Loc::new(sp, TypeParam::Number(n)));
            }
            Some(TK::Identifier | TK::IdentifierQuoted(_)) => {
                let sp = i.first().unwrap().span;
                any.parse_next(i)?;
                params.push(Loc::new(sp, TypeParam::Ident(sp)));
            }
            _ => break,
        }
        if opt(tk(TK::Comma)).parse_next(i)?.is_none() {
            break;
        }
    }
    opt(tk(TK::RightParen)).parse_next(i)?;
    Ok(DataType::Parameterized { name, params })
}

fn parse_case(i: &mut Tokens, start: usize) -> P<Expr> {
    let operand = if peek_kind(i) != Some(TK::Keyword(KW::When)) {
        Some(Box::new(spanned(expr).parse_next(i)?))
    } else {
        None
    };
    let mut when_clauses = Vec::new();
    while opt(kw(KW::When)).parse_next(i)?.is_some() {
        let when = spanned(expr).parse_next(i)?;
        kw(KW::Then).parse_next(i)?;
        let then = spanned(expr).parse_next(i)?;
        when_clauses.push(WhenClause { when, then });
    }
    let else_clause = if opt(kw(KW::Else)).parse_next(i)?.is_some() {
        Some(Box::new(spanned(expr).parse_next(i)?))
    } else {
        None
    };
    kw(KW::End).parse_next(i)?;
    let case = Case {
        operand,
        when_clauses,
        else_clause,
    };
    Ok(Expr::Case(Loc::new((start, end_from(i, start)), case)))
}

fn parse_quantified(i: &mut Tokens, start: usize) -> P<Expr> {
    let quantifier = match peek_kind(i) {
        Some(TK::Keyword(KW::Any)) => Quantifier::Any,
        Some(TK::Keyword(KW::Some)) => Quantifier::Some,
        Some(TK::Keyword(KW::All)) if peek_nth(i, 1) == Some(TK::LeftParen) => Quantifier::All,
        _ => return fail(i),
    };
    any.parse_next(i)?;
    tk(TK::LeftParen).parse_next(i)?;
    let expr = if is_query_start(i) {
        let q = spanned(query).parse_next(i)?;
        Box::new(Loc::new(q.span, Expr::Subquery(Box::new(q))))
    } else {
        Box::new(spanned(expr).parse_next(i)?)
    };
    opt(tk(TK::RightParen)).parse_next(i)?;
    Ok(Expr::Quantified(Loc::new(
        (start, end_from(i, start)),
        Quantified { quantifier, expr },
    )))
}

fn parse_typed_literal(i: &mut Tokens, start: usize) -> P<Loc<Literal>> {
    let data_type = match peek_kind(i) {
        Some(TK::Keyword(KW::Date)) => TypedLiteralKind::Date,
        Some(TK::Keyword(KW::Time)) => TypedLiteralKind::Time,
        Some(TK::Keyword(KW::Timestamp)) => TypedLiteralKind::Timestamp,
        Some(TK::Keyword(KW::Interval)) => TypedLiteralKind::Interval,
        _ => return fail(i),
    };
    any.parse_next(i)?;
    let value = tk(TK::Str).parse_next(i)?.span;
    let lit = Literal::TypedString { data_type, value };
    Ok(Loc::new((start, end_from(i, start)), lit))
}

fn parse_func_call(i: &mut Tokens, name: Loc<QualifiedName>, start: usize) -> P<Expr> {
    let distinct = opt(kw(KW::Distinct)).parse_next(i)?.is_some();
    let args_start = cur_start(i);
    let arglist = args(i)?;
    opt(tk(TK::RightParen)).parse_next(i)?;
    let args = Loc::new(Span::new(args_start, end_from(i, args_start)), arglist);
    let filter = if opt(kw(KW::Filter)).parse_next(i)?.is_some() {
        Some(Box::new(filter_predicate(i)?))
    } else {
        None
    };
    let span = Span::new(name.span.start, end_from(i, name.span.end));
    let e = if opt(kw(KW::Over)).parse_next(i)?.is_some() {
        let over = parse_window_ref(i)?;
        Expr::Over(Loc::new(
            span,
            Over {
                name,
                args,
                over,
                filter,
            },
        ))
    } else {
        Expr::FunctionCall(Loc::new(
            span,
            FunctionCall {
                name,
                distinct,
                args,
                filter,
            },
        ))
    };
    parse_postfix(i, start, e)
}

/// `FILTER ( WHERE <predicate> )` body (the `FILTER` keyword is already eaten).
fn filter_predicate(i: &mut Tokens) -> P<Loc<Expr>> {
    tk(TK::LeftParen).parse_next(i)?;
    opt(kw(KW::Where)).parse_next(i)?;
    let pred = spanned(expr).parse_next(i)?;
    opt(tk(TK::RightParen)).parse_next(i)?;
    Ok(pred)
}

fn parse_literal(i: &mut Tokens) -> P<Loc<Literal>> {
    any.verify_map(|t: &Token| {
        let lit = match t.kind {
            TK::Number => Literal::Number(t.text.parse().ok()?),
            TK::Float => Literal::Float(t.text.parse().ok()?),
            TK::Str => Literal::String(t.span),
            TK::Keyword(KW::Null) => Literal::Null,
            TK::Keyword(KW::True) => Literal::Boolean(Boolean::True),
            TK::Keyword(KW::False) => Literal::Boolean(Boolean::False),
            TK::Keyword(KW::Unknown) => Literal::Boolean(Boolean::Unknown),
            _ => return None,
        };
        Some(Loc::new(t.span, lit))
    })
    .parse_next(i)
}

fn qualified_name(i: &mut Tokens) -> P<QualifiedName> {
    Ok(QualifiedName {
        parts: sep_list(TK::Dot, name_part).parse_next(i)?,
    })
}

fn name_part(i: &mut Tokens) -> P<NamePart> {
    if let Some(id) = opt(ident).parse_next(i)? {
        return Ok(NamePart::Ident(id));
    }
    if opt(op(OpTag::Mul)).parse_next(i)?.is_some() {
        return Ok(NamePart::Star);
    }
    if let Some(TK::Keyword(w)) = peek_kind(i)
        && !i.state.is_reserved(w)
    {
        let sp = i.first().unwrap().span;
        any.parse_next(i)?;
        return Ok(NamePart::Ident(sp));
    }
    fail(i)
}

// ============================================================================
// Atoms & combinators
// ============================================================================

/// Match a single token of an exact [`TokenKind`] (punctuation, EOF, ...).
fn tk<'a>(kind: TK) -> impl Parser<Tokens<'a>, &'a Token<'a>, PErr> {
    any.verify(move |t: &&Token| t.kind == kind)
}

/// Match a keyword token.
fn kw<'a>(k: KW) -> impl Parser<Tokens<'a>, &'a Token<'a>, PErr> {
    tk(TK::Keyword(k))
}

/// Match an operator token by its semantic tag.
fn op<'a>(tag: OpTag) -> impl Parser<Tokens<'a>, &'a Token<'a>, PErr> {
    any.verify(move |t: &&Token| matches!(t.kind, TK::Operator(o) if o.semantic_tag == tag))
}

/// Match an identifier (plain or quoted), yielding its span.
fn ident(i: &mut Tokens) -> P<Identifier> {
    any.verify(|t: &&Token| matches!(t.kind, TK::Identifier | TK::IdentifierQuoted(_)))
        .map(|t: &Token| t.span)
        .parse_next(i)
}

/// Match an identifier or any keyword as a column-alias span.
fn ident_or_keyword(i: &mut Tokens) -> P<Identifier> {
    any.verify(|t: &&Token| {
        matches!(
            t.kind,
            TK::Identifier | TK::IdentifierQuoted(_) | TK::Keyword(_)
        )
    })
    .map(|t: &Token| t.span)
    .parse_next(i)
}

/// Optional table/column alias, with or without the `AS` keyword.
///
/// Always succeeds. A bare `AS` (no following identifier) is still consumed and
/// yields `None`, matching the cursor needs of `... AS ^` completions.
fn alias(i: &mut Tokens) -> P<Option<Loc<Identifier>>> {
    if opt(kw(KW::As)).parse_next(i)?.is_some() {
        return opt(spanned(ident)).parse_next(i);
    }
    match peek_kind(i) {
        Some(TK::Identifier | TK::IdentifierQuoted(_)) => Ok(Some(spanned(ident).parse_next(i)?)),
        _ => Ok(None),
    }
}

/// Run `p`, attaching the consumed source span to its output.
fn spanned<'a, O>(
    mut p: impl Parser<Tokens<'a>, O, PErr>,
) -> impl Parser<Tokens<'a>, Loc<O>, PErr> {
    move |i: &mut Tokens<'a>| {
        let start = cur_start(i);
        let v = p.parse_next(i)?;
        Ok(Loc::new((start, end_from(i, start)), v))
    }
}

/// One-or-more `item`s separated by `sep`; captures separator spans and
/// tolerates a trailing separator.
fn sep_list<'a, O>(
    sep: TK, mut item: impl Parser<Tokens<'a>, O, PErr>,
) -> impl Parser<Tokens<'a>, DelimitedList<Loc<O>>, PErr> {
    move |i: &mut Tokens<'a>| {
        let mut items = vec![spanned(item.by_ref()).parse_next(i)?];
        let mut seps = Vec::new();
        while let Some(t) = opt(tk(sep)).parse_next(i)? {
            seps.push(Loc::new(t.span, sep));
            match opt(spanned(item.by_ref())).parse_next(i)? {
                Some(x) => items.push(x),
                None => break,
            }
        }
        Ok(DelimitedList { items, seps })
    }
}

/// Comma-separated [`sep_list`].
fn comma_list<'a, O>(
    item: impl Parser<Tokens<'a>, O, PErr>,
) -> impl Parser<Tokens<'a>, DelimitedList<Loc<O>>, PErr> {
    sep_list(TK::Comma, item)
}

/// Wrap `inner` in parentheses, tolerating a missing closing paren.
fn parens<'a, O>(inner: impl Parser<Tokens<'a>, O, PErr>) -> impl Parser<Tokens<'a>, O, PErr> {
    delimited(tk(TK::LeftParen), inner, opt(tk(TK::RightParen)))
}

/// Comma-separated expression list inside a call/paren; empty if at `)`.
fn args(i: &mut Tokens) -> P<DelimitedList<Loc<Expr>>> {
    if peek_kind(i) == Some(TK::RightParen) {
        return Ok(DelimitedList::default());
    }
    Ok(opt(comma_list(expr)).parse_next(i)?.unwrap_or_default())
}

/// Run `f`, restoring the input position if it fails (backtracking try).
fn try_with<'a, O>(i: &mut Tokens<'a>, f: impl FnOnce(&mut Tokens<'a>) -> P<O>) -> Option<O> {
    let cp = i.checkpoint();
    match f(i) {
        Ok(v) => Some(v),
        Err(_) => {
            i.reset(&cp);
            None
        }
    }
}

/// Start byte of the next token, falling back to the previous token's end.
fn cur_start(i: &Tokens<'_>) -> usize {
    i.first()
        .map(|t| t.span.start)
        .unwrap_or_else(|| i.previous_tokens().next().map(|t| t.span.end).unwrap_or(0))
}

/// End byte of the most recently consumed token, or `fallback`.
fn end_from(i: &Tokens<'_>, fallback: usize) -> usize {
    i.previous_tokens()
        .next()
        .map(|t| t.span.end)
        .unwrap_or(fallback)
}

fn peek_kind(i: &Tokens<'_>) -> Option<TK> {
    i.first().map(|t| t.kind)
}

fn peek_nth(i: &Tokens<'_>, n: usize) -> Option<TK> {
    i.get(n).map(|t| t.kind)
}

/// Does the input start a (sub)query: `SELECT ...` or `WITH ...`?
fn is_query_start(i: &Tokens<'_>) -> bool {
    matches!(peek_kind(i), Some(TK::Keyword(KW::Select | KW::With)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::DialectKind;
    use crate::lex;

    fn parse(sql: &str) -> Option<Loc<Statement>> {
        let spec = DialectKind::Postgres.spec();
        parse_statement(&lex::lex(spec, sql), spec)
    }

    fn is_query(sql: &str) -> bool {
        matches!(
            parse(sql),
            Some(Loc {
                item: Statement::Query(_),
                ..
            })
        )
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
        assert!(is_query("SELECT * FROM a JOIN b ON a.id = b.id"));
        assert!(is_query("SELECT * FROM a LEFT OUTER JOIN b ON a.id = b.id"));
        assert!(is_query("SELECT * FROM a NATURAL JOIN b"));
        assert!(is_query("SELECT * FROM a CROSS JOIN b"));
    }

    #[test]
    fn union() {
        assert!(matches!(
            parse("SELECT 1 UNION ALL SELECT 2"),
            Some(Loc { item: Statement::Query(q), .. })
                if q.body.as_ref().map(|b| !b.set_ops.is_empty()).unwrap_or(false)
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
    }

    #[test]
    fn case_expr() {
        assert!(parse("SELECT CASE WHEN x > 1 THEN 'a' ELSE 'b' END FROM t").is_some());
        assert!(parse("SELECT CASE x WHEN 1 THEN 'a' WHEN 2 THEN 'b' END").is_some());
    }

    #[test]
    fn window_function() {
        assert!(parse("SELECT rank() OVER (PARTITION BY a ORDER BY b) FROM t").is_some());
        assert!(parse("SELECT sum(x) OVER w FROM t WINDOW w AS (ORDER BY y)").is_some());
    }

    #[test]
    fn aggregate_filter() {
        assert!(parse("SELECT count(*) FILTER (WHERE x > 0) FROM t").is_some());
    }

    #[test]
    fn subquery_in_from() {
        assert!(parse("SELECT * FROM (SELECT 1) sub").is_some());
    }

    #[test]
    fn exists_subquery() {
        assert!(parse("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM s)").is_some());
    }

    #[test]
    fn quantified() {
        assert!(parse("SELECT * FROM t WHERE x = ANY (SELECT id FROM s)").is_some());
    }

    #[test]
    fn group_by_variants() {
        assert!(parse("SELECT a, SUM(c) FROM t GROUP BY ROLLUP(a, b)").is_some());
        assert!(parse("SELECT a, SUM(c) FROM t GROUP BY CUBE(a, b)").is_some());
        assert!(parse("SELECT a FROM t GROUP BY GROUPING SETS((a, b), (a), ())").is_some());
        assert!(parse("SELECT a FROM t GROUP BY a, ROLLUP(b, c)").is_some());
    }

    #[test]
    fn insert_variants() {
        let insert = |s| {
            matches!(
                parse(s),
                Some(Loc {
                    item: Statement::Insert(_),
                    ..
                })
            )
        };
        assert!(insert("INSERT INTO t (a, b) VALUES (1, 2)"));
        assert!(insert("INSERT INTO t VALUES (1), (2), (3)"));
        assert!(insert("INSERT INTO t SELECT * FROM s"));
        assert!(insert("INSERT INTO t DEFAULT VALUES"));
        assert!(insert("INSERT INTO t (a) VALUES (1) RETURNING *"));
    }

    #[test]
    fn update_variants() {
        let update = |s| {
            matches!(
                parse(s),
                Some(Loc {
                    item: Statement::Update(_),
                    ..
                })
            )
        };
        assert!(update("UPDATE t SET a = 1 WHERE id = 1"));
        assert!(update("UPDATE t SET a = 1, b = 2, c = 3"));
        assert!(update("UPDATE t SET a = s.a FROM s WHERE t.id = s.id"));
        assert!(update("UPDATE t SET a = 1 RETURNING *"));
    }

    #[test]
    fn delete_variants() {
        let delete = |s| {
            matches!(
                parse(s),
                Some(Loc {
                    item: Statement::Delete(_),
                    ..
                })
            )
        };
        assert!(delete("DELETE FROM t WHERE id = 1"));
        assert!(delete("DELETE FROM t USING s WHERE t.id = s.id"));
        assert!(delete("DELETE FROM t WHERE id = 1 RETURNING *"));
    }

    #[test]
    fn values_statement() {
        assert!(parse("VALUES (1, 2), (3, 4)").is_some());
        assert!(parse("WITH v AS (VALUES (1), (2)) SELECT * FROM v").is_some());
    }

    #[test]
    fn empty_and_partial_input() {
        assert!(matches!(
            parse(""),
            Some(Loc {
                item: Statement::Partial(_),
                ..
            })
        ));
        assert!(matches!(
            parse("foo"),
            Some(Loc {
                item: Statement::Partial(_),
                ..
            })
        ));
        // Incomplete predicate still yields a query (error-tolerant).
        assert!(is_query("SELECT * FROM t WHERE x ="));
    }

    #[test]
    fn limit_offset() {
        assert!(parse("SELECT * FROM t LIMIT 10 OFFSET 5").is_some());
        assert!(parse("SELECT * FROM t OFFSET 5 ROWS FETCH FIRST 10 ROWS ONLY").is_some());
    }
}
