use std::borrow::Cow;
use std::collections::HashSet;

use crate::complete::Completer;
use crate::complete::completion::Candidate;
use crate::complete::completion::CandidateKind;
use crate::complete::completion::CandidateSet;
use crate::complete::completion::ColumnCandidate;
use crate::complete::context::ClausePosition;
use crate::complete::context::Context;
use crate::complete::context::Location;
use crate::complete::context::Projection;
use crate::complete::context::ProjectionKind;
use crate::complete::context::QualifiedIdent;
use crate::schema;

pub struct ColumnProvider;
impl<'a> Completer<'a> for ColumnProvider {
    fn complete(&mut self, ctx: &mut Context<'a>, b: &mut CandidateSet<'a>) {
        let cols = discover_columns(ctx);
        let labeler = DefaultLabelRenderer;
        let detailr = DefaultDetailRenderer;

        for col in cols {
            let base = Candidate::new(CandidateKind::Column(ColumnCandidate {
                dt: col.dt,
                scope_alias: col.source_alias,
                ident: col.ident,
            }));
            let detail = detailr.detail(ctx, &col);
            let base = base.detail(detail);

            for label in labeler.labels(ctx, &col) {
                b.push(base.clone().label(label.into_owned()));
            }
        }
    }

    fn should_complete(&self, ctx: &Context<'a>) -> bool {
        should_complete(ctx)
    }
}

fn should_complete<'a>(ctx: &Context<'a>) -> bool {
    use ClausePosition as CP;
    use Location as L;
    match (&ctx.cursor.location, ctx.clause.pos) {
        // Keyword no space
        (L::Keyword(_), _) => return false,
        // After a space and an ident
        (L::Space(inner), Some(CP::ExprLeft)) if matches!(**inner, L::Ident) => false,
        // ExprLeft
        (_, Some(CP::ExprLeft)) => true,
        _ => false,
    }
}

#[derive(Debug, Clone)]
struct LogicalColumn<'a> {
    dt: Option<schema::DataType>,
    source_alias: Option<&'a str>, // table/cte alias in scope
    ident: QualifiedIdent<'a>,     // for rendering variants
}

fn discover_columns<'a>(ctx: &Context<'a>) -> Vec<LogicalColumn<'a>> {
    let available_columns = ctx
        .resolved_scope()
        .available()
        .filter(|p| p.is_referenceable())
        .map(|p| map_projection_to_logical(*p))
        .collect::<Vec<_>>();

    if available_columns.len() == 0 {
        return ctx
            .schema()
            .get_columns()
            .into_iter()
            .map(|c| LogicalColumn {
                dt: Some(c.data_type),
                source_alias: c.table_name.as_ref().map(|s| s.as_str()),
                ident: QualifiedIdent::from(c),
            })
            .collect::<Vec<_>>();
    }
    available_columns
}

fn map_projection_to_logical<'a>(p: Projection<'a>) -> LogicalColumn<'a> {
    let ident = match p.kind {
        ProjectionKind::Column { schema_column, .. } => schema_column.map(QualifiedIdent::from),
        ProjectionKind::TableFunction { .. } => None,
        ProjectionKind::ScalarFunction { .. }
        | ProjectionKind::Literal { .. }
        | ProjectionKind::Expression { .. }
        | ProjectionKind::Unresolved => None,
    };

    LogicalColumn {
        dt: p.data_type(),
        source_alias: p.label.table(),
        ident: p.label,
    }
}

trait ColumnLabelRenderer {
    fn labels<'a>(&self, ctx: &Context<'a>, col: &LogicalColumn<'a>) -> Vec<Cow<'a, str>>;
}

struct DefaultLabelRenderer;

impl ColumnLabelRenderer for DefaultLabelRenderer {
    fn labels<'a>(&self, ctx: &Context<'a>, col: &LogicalColumn<'a>) -> Vec<Cow<'a, str>> {
        let mut out = Vec::with_capacity(4);

        // Unqualified
        out.push(Cow::Borrowed(col.ident.name()));

        // alias.col
        if let Some(a) = col.source_alias {
            out.push(Cow::Owned(format!("{a}.{}", col.ident.name())));
        }

        // table.col (if table is known & not equal to alias)
        if let Some(t) = col.ident.table() {
            out.push(Cow::Owned(format!("{t}.{}", col.ident.name())));
        }

        // schema.table.col
        if let (Some(s), Some(t)) = (col.ident.schema(), col.ident.table()) {
            out.push(Cow::Owned(format!("{s}.{t}.{}", col.ident.name())));
        }

        // Optional: database.schema.table.col (if you support it)
        // if let (Some(d), Some(s), Some(t)) = (col.ident_parts.database, col.ident_parts.schema, col.ident_parts.table) {
        //     out.push(Cow::Owned(format!("{d}.{s}.{t}.{}", col.ident_parts.column)));
        // }

        // Simple dedup (alias==table etc.)
        let mut seen = HashSet::with_capacity(out.len());
        out.retain(|l| seen.insert(l.as_ref().to_string()));
        out
    }
}

trait ColumnDetailRenderer {
    fn detail<'a>(&self, _ctx: &Context<'a>, col: &LogicalColumn<'a>) -> String;
}

struct DefaultDetailRenderer;

impl ColumnDetailRenderer for DefaultDetailRenderer {
    fn detail<'a>(&self, _ctx: &Context<'a>, col: &LogicalColumn<'a>) -> String {
        let mut parts = String::new();
        if let Some(dt) = col.dt {
            parts.push_str(dt.to_string().as_str());
            parts.push_str(" • ");
        }
        if let Some(s) = col.ident.schema() {
            parts.push_str(s);
            parts.push('.');
        }
        if let Some(t) = col.ident.table() {
            parts.push_str(t);
            parts.push('.');
        }
        parts.push_str(col.ident.name());
        parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::ansi;
    use crate::test_complete;
    use crate::test_util::get_caret_cursor;

    fn assert_not_complete(input: &str) {
        let (sql, cursor) = get_caret_cursor(input);
        let schema = schema::Cache::default();
        let ctx = Context::build(&ansi::SPEC, &schema, &sql, cursor).unwrap();
        let should_complete = should_complete(&ctx);
        if should_complete {
            println!("clause: {:#?}", ctx.clause);
            println!("cursor: {:#?}", ctx.cursor);
        }
        assert!(!should_complete, "should not complete for: {input}");
    }

    #[test]
    fn completes_at_appropriate_locations() {
        assert_not_complete("SELECT^");
        assert_not_complete("SELECT a ^");
        assert_not_complete("SELECT a as^");
        assert_not_complete("SELECT a as b^");
        assert_not_complete("SELECT a as b ^");
        assert_not_complete("SELECT a FROM^");
        assert_not_complete("SELECT a FROM ^");
        assert_not_complete("SELECT a FROM a^");
        assert_not_complete("SELECT a FROM a ^");
        assert_not_complete("SELECT a FROM a t^");
        assert_not_complete("SELECT a FROM a t, ^");
        assert_not_complete("SELECT a FROM a t,^");
        assert_not_complete("SELECT a FROM a t, ^");
        assert_not_complete("SELECT a FROM b WHERE a ^");
        assert_not_complete("SELECT a FROM b WHERE a =^");
        assert_not_complete("SELECT a FROM b WHERE a = b^");
    }

    #[test]
    fn include_both_qualified_and_unqualified_names() {
        let schema = schema::CacheBuilder::new()
            .table_in("users", "public", None)
            .column_in("id", schema::DataType::Integer, "users", "public", None)
            .build();

        // Aliased
        test_complete!("SELECT ^ FROM users u", "SELECT ^ FROM (SELECT id FROM users) u" => {
            completers: [ColumnProvider],
            schemas: [schema.clone()],
            contains: ["id", "u.id"],
        });

        // Unaliased
        test_complete!("SELECT ^", "SELECT ^ FROM users" => {
            completers: [ColumnProvider],
            schemas: [schema],
            contains: ["id", "users.id", "public.users.id"],
        });
    }

    #[test]
    fn include_columns_from_table_functions() {
        use schema::DataType::*;
        let schema = schema::CacheBuilder::new()
            .table_function("generate_series", &[Integer, Integer], vec![("i", Integer)])
            .build();

        test_complete!("SELECT ^ FROM generate_series(1, 10)" => {
            completers: [ColumnProvider],
            schemas: [schema.clone()],
            contains: ["i"],
        });
        test_complete!("SELECT ^ FROM generate_series(1, 10) u" => {
            completers: [ColumnProvider],
            schemas: [schema],
            contains: ["i", "u.i"],
        });
    }

    #[test]
    fn include_columns_from_literal_scalar_functions() {
        use schema::DataType::*;
        let schema = schema::CacheBuilder::new()
            .scalar_function("upper", &[Text], Text)
            .build();
        test_complete!("SELECT ^ FROM (SELECT 1 + 2 as a, upper('hello') as b) u" => {
            completers: [ColumnProvider],
            schemas: [schema],
            contains: ["a", "b", "u.a", "u.b"],
        });
    }
}
