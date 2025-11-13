use crate::complete::Completer;
use crate::complete::candidate::CandidateBuilder;
use crate::complete::candidate::CandidateSet;
use crate::complete::context::ClausePosition;
use crate::complete::context::Context;
use crate::complete::context::Location;
use crate::complete::context::QualifiedIdent;
use crate::schema;

#[derive(Debug, Default)]
pub struct ColumnProvider;

impl Completer for ColumnProvider {
    fn complete<'a>(&mut self, ctx: &mut Context<'a>, b: &mut CandidateSet<'a>) {
        let cols = discover_columns(ctx);
        let detailer = DefaultDetailRenderer;
        let qualifier = ctx.cursor().qualifier.as_ref();

        for col in cols {
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
        // Keyword no space
        (L::Keyword(_), _) => false,
        // After a space and an ident - block column completions
        (L::Space(inner), Some(CP::ExprLeft)) if matches!(**inner, L::Ident) => false,
        // After comma without space - block column completions
        (L::Comma, Some(CP::ExprLeft)) => false,
        // After comma and space - allow column completions
        (L::Space(inner), Some(CP::ExprLeft)) if matches!(**inner, L::Comma) => true,
        // Other ExprLeft positions - allow column completions
        (_, Some(CP::ExprLeft)) => true,
        _ => false,
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
    use crate::test_complete;
    use crate::test_utils::get_caret_cursor;

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
