use crate::complete::Completer;
use crate::complete::candidate::CandidateBuilder;
use crate::complete::candidate::CandidateSet;
use crate::complete::context::ClauseKind;
use crate::complete::context::ClausePosition;
use crate::complete::context::Context;
use crate::complete::context::Location;
use crate::complete::context::QualifiedIdent;
use crate::lex::OpTag;
use crate::schema;

#[derive(Debug, Default)]
pub struct ColumnProvider;

impl Completer for ColumnProvider {
    fn complete<'a>(&mut self, ctx: &mut Context<'a>, b: &mut CandidateSet<'a>) {
        let cols = discover_columns(ctx);
        let detailer = DefaultDetailRenderer;
        let qualifier = ctx.cursor().qualifier.as_ref();
        let order_by_used = ctx.scope().order_by();

        for col in cols {
            if order_by_used.iter().any(|ident| col.ident.matches(ident)) {
                continue;
            }

            // If we have a qualifier (e.g., "users.^" or "u.^"), filter columns
            if let Some(qual_parts) = qualifier {
                // For now, handle single-part qualifiers (table name or alias)
                if qual_parts.len() == 1 {
                    let qual = qual_parts[0];

                    // Check if this column matches the qualifier in any of its variants
                    // This handles cases like "users.id" matching "users" or "u.id" matching "u"
                    let matches_qualifier = col.ident.variants().iter().any(|variant| {
                        // Match if the variant has the qualifier as parent and no schema
                        // (e.g., "users" in "users.id", or "u" in "u.id")
                        variant.parent == Some(qual) && variant.schema.is_none()
                    });

                    if matches_qualifier {
                        // Only suggest unqualified name when after "table."
                        let name_label = QualifiedIdent::from_str(
                            crate::complete::context::IdentKind::Column,
                            col.ident.name(),
                        );
                        b.push(
                            CandidateBuilder::column(name_label, col.ident, col.dt)
                                .detail(detailer.detail(ctx, &col))
                                .build(),
                        );
                    }
                }
                // Skip to next column when we have a qualifier
                continue;
            }

            // No qualifier - suggest all variants
            for label in col.ident.variants() {
                b.push(
                    CandidateBuilder::column(label, col.ident, col.dt)
                        .detail(detailer.detail(ctx, &col))
                        .build(),
                );
            }
        }
    }

    fn should_complete<'a>(&self, ctx: &Context<'a>) -> bool {
        should_complete(ctx)
    }
}

fn should_complete<'a>(ctx: &Context<'a>) -> bool {
    use ClausePosition as CP;
    use Location as L;

    match (&ctx.cursor().location, ctx.clause().pos) {
        // Keywords or identifiers in-progress
        (L::Keyword(_), _) => false,
        (L::Ident, _) => false,
        // Operators and literals shouldn't trigger column completion
        (L::Operator(tag), _) if !matches!(tag, OpTag::And | OpTag::Or) => false,
        (L::Literal, _) => false,
        // After a space and an ident/operator/literal - block column completions
        (L::Space(inner), _)
            if matches!(**inner, L::Ident | L::Literal)
                || matches!(
                    **inner,
                    L::Operator(tag) if !matches!(tag, OpTag::And | OpTag::Or)
                ) =>
        {
            false
        }
        // After comma without space - block column completions
        (L::Comma, Some(CP::ExprLeft)) => false,
        // Allow completing after logical operators (AND/OR)
        (L::Space(inner), Some(CP::ExprRight))
            if matches!(
                **inner,
                L::Operator(tag) if matches!(tag, OpTag::And | OpTag::Or)
            ) =>
        {
            true
        }
        // After comma and space - allow column completions
        (L::Space(inner), Some(CP::ExprLeft)) if matches!(**inner, L::Comma) => true,
        // After dot - allow column completions (for qualified names like table.^)
        (L::Dot, _) => true,
        // Other ExprLeft positions - allow column completions
        (_, Some(CP::ExprLeft)) => true,
        // Fallback: allow columns after clause keywords like WHERE/AND
        (location, _) => {
            let allow = match location {
                L::Space(inner) => match &**inner {
                    L::Keyword(_) | L::Comma | L::Paren => true,
                    L::Operator(tag) if matches!(tag, OpTag::And | OpTag::Or) => true,
                    _ => false,
                },
                _ => false,
            };
            allow
                && matches!(
                    ctx.clause().kind,
                    ClauseKind::Where
                        | ClauseKind::Select
                        | ClauseKind::GroupBy
                        | ClauseKind::OrderBy
                )
        }
    }
}

#[derive(Debug, Clone)]
struct LogicalColumn<'a> {
    dt: Option<schema::DataType>,
    ident: QualifiedIdent<'a>, // for rendering variants
}

fn discover_columns<'a>(ctx: &Context<'a>) -> Vec<LogicalColumn<'a>> {
    let avail = ctx.scope().available().filter(|p| p.is_referenceable());
    let cols: Vec<_> = avail
        .map(|p| LogicalColumn {
            dt: p.data_type(),
            ident: p.label,
        })
        .collect();

    if !cols.is_empty() {
        return cols;
    }

    return ctx
        .schema()
        .get_columns()
        .into_iter()
        .map(|c| LogicalColumn {
            dt: Some(c.data_type),
            ident: QualifiedIdent::from(c),
        })
        .collect();
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
    use crate::test_utils::ScenarioComp;
    use crate::test_utils::get_caret_cursor;
    use crate::test_utils::users_schema;

    fn assert_not_complete(input: &str) {
        let (sql, cursor) = get_caret_cursor(input);
        let schema = schema::Cache::default();
        let ctx = Context::build(&ansi::SPEC, &schema, &sql, cursor).unwrap();
        let should_complete = should_complete(&ctx);
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

    fn scenario() -> ScenarioComp {
        ScenarioComp::default()
            .completer(ColumnProvider)
            .spec(ansi::SPEC.clone())
    }

    #[test]
    fn include_both_qualified_and_unqualified_names() {
        let schema = schema::CacheBuilder::new()
            .table_in("users", "public", None)
            .column_in("id", schema::DataType::Integer, "users", "public", None)
            .build();

        // Aliased
        scenario()
            .with(schema.clone())
            .query([
                "SELECT ^ FROM users u",
                "SELECT ^ FROM (SELECT id FROM users) u",
            ])
            .contains(["id", "u.id"])
            .run();
    }

    #[test]
    fn include_columns_from_table_functions() {
        use schema::DataType::*;

        let schema = schema::CacheBuilder::new()
            .table_function("generate_series", &[Integer, Integer], vec![("i", Integer)])
            .build();

        scenario()
            .with(schema.clone())
            .query("SELECT ^ FROM generate_series(1, 10)")
            .contains(["i"])
            .run();

        scenario()
            .with(schema)
            .query("SELECT ^ FROM generate_series(1, 10) u")
            .contains(["i", "u.i"])
            .run();
    }

    #[test]
    fn include_columns_from_literal_scalar_functions() {
        use schema::DataType::*;
        let schema = schema::CacheBuilder::new()
            .scalar_function("upper", &[Text], Text)
            .build();
        scenario()
            .with(schema)
            .query("SELECT ^ FROM (SELECT 1 + 2 as a, upper('hello') as b) u")
            .contains(["a", "b", "u.a", "u.b"])
            .run();
    }

    #[test]
    fn completes_after_logical_operator() {
        let schema = users_schema();
        let (sql, cursor) = get_caret_cursor("SELECT * FROM users WHERE name = 'John' AND ^");
        let ctx = Context::build(&ansi::SPEC, &schema, &sql, cursor).unwrap();
        assert!(
            should_complete(&ctx),
            "expected columns to complete after logical operator, got location {:?}",
            ctx.cursor().location
        );
    }

    #[test]
    fn completes_in_order_by_clause() {
        let schema = users_schema();
        let (sql, cursor) = get_caret_cursor("SELECT * FROM users ORDER BY ^");
        let ctx = Context::build(&ansi::SPEC, &schema, &sql, cursor).unwrap();
        assert!(
            should_complete(&ctx),
            "expected columns to complete in ORDER BY, got location {:?} with clause {:?}",
            ctx.cursor().location,
            ctx.clause().kind
        );
    }
}
