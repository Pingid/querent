use crate::complete::Completer;
use crate::complete::candidate::CandidateBuilder;
use crate::complete::candidate::CandidateSet;
use crate::complete::context::ClauseKind;
use crate::complete::context::Context;
use crate::complete::context::IdentKind;
use crate::complete::context::Location;
use crate::complete::context::QualifiedIdent;
use crate::lex::Keyword;
use crate::lex::TokenKind;

pub struct TableProvider;
impl<'a> Completer<'a> for TableProvider {
    fn complete(&mut self, ctx: &mut Context<'a>, b: &mut CandidateSet<'a>) {
        // Check if we're completing after a schema qualifier (e.g., "public.^")
        let qualifier = ctx.cursor().qualifier.as_ref();

        for table in ctx.schema().get_tables() {
            let label = QualifiedIdent::from(table);

            // If we have a qualifier, only suggest tables from that schema
            if let Some(qual_parts) = qualifier {
                // For now, we only handle single-part qualifiers (schema name)
                if qual_parts.len() == 1 {
                    if let Some(schema) = label.schema() {
                        if schema == qual_parts[0] {
                            // Only suggest unqualified name when after "schema."
                            let name_ident =
                                QualifiedIdent::from_str(IdentKind::Table, label.name());
                            b.push(CandidateBuilder::table(name_ident, label).build());
                        }
                    }
                }
            } else {
                // No qualifier - suggest all variants
                for variant in label.variants() {
                    b.push(CandidateBuilder::table(variant, label).build());
                }
            }
        }

        // CTEs are always unqualified
        if qualifier.is_none() {
            for cte in ctx.scope().ctes() {
                let label = QualifiedIdent::from_str(IdentKind::Table, cte);
                b.push(CandidateBuilder::table(label, label).build());
            }
        }
    }

    fn should_complete(&self, ctx: &Context<'a>) -> bool {
        match ctx.clause().kind {
            ClauseKind::From => match &ctx.cursor().location {
                // After FROM/JOIN keyword with space, or after comma
                Location::Space(inner) => {
                    matches!(
                        **inner,
                        Location::Keyword(Keyword::From)
                            | Location::Keyword(Keyword::Join)
                            | Location::Comma
                    )
                }
                // After a dot (e.g., "public.^")
                Location::Dot => true,
                // After FROM/JOIN and an identifier being typed
                Location::Ident
                    if ctx.cursor().preceding_matches([
                        TokenKind::Keyword(Keyword::From),
                        TokenKind::Identifier,
                    ]) || ctx.cursor().preceding_matches([
                        TokenKind::Keyword(Keyword::Join),
                        TokenKind::Identifier,
                    ]) =>
                {
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}
