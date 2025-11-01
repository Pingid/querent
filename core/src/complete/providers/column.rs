use crate::complete::completion::Candidate;
use crate::complete::completion::CandidateKind;
use crate::complete::completion::CandidateSet;
use crate::complete::completion::ColumnCandidate;
use crate::complete::context::BindingId;
use crate::complete::context::BindingKind;
use crate::complete::context::BoundColumn;
use crate::complete::context::Context;
use crate::complete::context::NamePath;
use crate::complete::context::Origin;
use crate::complete::context::RelationBinding;
use crate::complete::context::ResolvedColumnSource;
use crate::complete::context::ScopeGraph;
use crate::schema;

pub fn complete<'a>(ctx: &mut Context<'a>, builder: &mut CandidateSet<'a>) {
    // println!("ctx: {:#?}", ctx.resolved_scope);
    // println!("\n\nGRAPH: \n{:#?}\n\n", ctx.scope.graph);
    // let available = scope_columns(ctx.schema, &ctx.scope.graph, None);
    // println!("\n\navailable: \n{:#?}\n\n", available);
    // // Find all exposed columns from CTE's, FROM tables/subqueries, etc.
    // let available = ctx.scope.available_columns().clone();

    // // If no columns are available, add all columns from the schema.
    // if available.is_empty() {
    //     for col in ctx.schema.get_columns().iter() {
    //         let candidate = Candidate::new(
    //             col.column_name.clone(),
    //             CandidateKind::Column(ColumnCandidate::Schema(col)),
    //         );
    //         builder.with(candidate);
    //     }
    // }

    // // Add all available columns to the list.
    // for col in available {
    //     let candidate = Candidate::new(
    //         col.name,
    //         match col.source {
    //             ResolvedColumnSource::Schema(c) => {
    //                 CandidateKind::Column(ColumnCandidate::Schema(c))
    //             }
    //             ResolvedColumnSource::Literal { ty } => {
    //                 CandidateKind::Column(ColumnCandidate::Unresolved {
    //                     dt: Some(ty),
    //                     name: &col.name,
    //                 })
    //             }
    //             ResolvedColumnSource::Unresolved(qualifier) => {
    //                 CandidateKind::Column(ColumnCandidate::Unresolved {
    //                     dt: None,
    //                     name: &col.name,
    //                 })
    //             }
    //             ResolvedColumnSource::Cte => CandidateKind::Column(ColumnCandidate::Cte {
    //                 cte: &col.source_alias,
    //                 dt: None,
    //                 name: &col.name,
    //             }),
    //         },
    //     );
    //     builder.with(candidate);
    // }

    // // Filter out columns that don't match the qualifier.
    // cols.retain(|col| col.matches_qualifier(&self.cursor.qualifier));

    // cols
}

fn scope_columns<'a>(
    schema: &'a schema::Cache, scope: &ScopeGraph<'a>, scope_alias: Option<&'a str>,
) -> Vec<ColumnCandidate<'a>> {
    let mut resolved = Vec::new();
    for relation in scope.bindings.values() {
        resolved.extend(relation_projections(schema, scope, relation, scope_alias));
    }
    resolved
}

fn scope_projections<'a>(
    schema: &'a schema::Cache, scope: &ScopeGraph<'a>, scope_alias: Option<&'a str>,
) -> Vec<ColumnCandidate<'a>> {
    let mut resolved = Vec::new();
    for b in scope.projected.iter() {
        resolved.extend(resolve_column_from_origin(
            schema,
            scope,
            scope_alias,
            b,
            b.alias,
        ));
    }
    resolved
}

fn relation_projections<'a>(
    schema: &'a schema::Cache, scope: &ScopeGraph<'a>, relation: &RelationBinding<'a>,
    scope_alias: Option<&'a str>,
) -> Vec<ColumnCandidate<'a>> {
    let mut resolved = Vec::new();

    match &relation.kind {
        BindingKind::Base(path) => {
            println!("\nBase: {:?}\n scope_alias: {:?}\n", path, scope_alias);
            // let qualifier = NamePath(path.0.clone());
            // resolved.extend(
            //     find_columns_in_schema(schema, &qualifier, None)
            //         .map(|c| ColumnCandidate::Schema(c)),
            // );
        }
        BindingKind::Subquery(scope) => {
            resolved.extend(scope_projections(schema, scope, relation.alias))
        }
        BindingKind::Cte { scope, name } => {
            resolved.extend(scope_projections(schema, scope, relation.alias))
        }
        other => {
            println!("\n\nrelation_projections Other: {:?}\n\n", other);
        }
    };
    resolved
}

fn resolve_projected<'a>(
    schema: &'a schema::Cache, scope: &ScopeGraph<'a>,
) -> Vec<ColumnCandidate<'a>> {
    let mut resolved = Vec::new();
    for b in scope.projected.iter() {}
    resolved
}

fn resolve_column_from_origin<'a>(
    schema: &'a schema::Cache, scope: &ScopeGraph<'a>, scope_alias: Option<&'a str>,
    column: &BoundColumn<'a>, name: Option<&'a str>,
) -> Vec<ColumnCandidate<'a>> {
    let mut resolved = Vec::new();
    println!("\n\nresolve_column_from_origin: {:?}\n\n", column);
    match &column.origin {
        Origin::BaseColumn { id, name } => {
            if let Some(relation) = scope.bindings.get(id) {
                println!("\n\nrelation: {:#?}\n\n", relation);
                // if let Some(c) = resolve_column(schema, name, relation, scope_alias) {
                //     resolved.push(c);
                // }
            }
        }
        Origin::Constant { dt, value } => {
            resolved.push(ColumnCandidate {
                dt: Some(*dt),
                col: None,
                name: column.alias.unwrap_or(value),
                scope_alias,
            });
        }
        Origin::Star { id } => {
            if let Some(id) = id {
                if let Some(relation) = scope.bindings.get(id) {
                    resolved.extend(relation_projections(schema, scope, &relation, scope_alias));
                }
            }
        }
        other => {
            println!(
                "\nUNSUPPORTED_ORIGIN: \nscope_alias: {:?} \ncolumn: {:#?} \nname: {:?}\n\n",
                scope_alias, column, name
            );
        }
    };
    resolved
}

fn resolve_column<'a>(
    schema: &'a schema::Cache, name: &NamePath<'a>, relation: &RelationBinding<'a>,
    scope_alias: Option<&'a str>,
) -> Option<ColumnCandidate<'a>> {
    match &relation.kind {
        BindingKind::Base(table) => {
            // let col = schema.get_columns().iter().find(|c| {
            //     table.match_column_as_table_name(c)
            //         && Some(&c.column_name.as_str()) == name.0.last()
            // });
            // Some(ColumnCandidate {
            //     dt: col.map(|c| c.data_type),
            //     col,
            //     scope_alias,
            //     name: None,
            // })
            None
        }
        other => {
            println!("\n\nresolve_column Other: {:?}\n\n", other);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::ansi;
    use crate::test_util::CompletionTest;
    use crate::test_util::CompletionTestResult;
    use crate::test_util::get_leaky_static_caret_cursor;
    use crate::test_util::users_posts_schema;

    fn ansi(sql: &str) -> CandidateSet<'static> {
        let (input, cursor) = get_leaky_static_caret_cursor(sql);
        let schema = Box::leak(Box::new(users_posts_schema()));
        let mut ctx = Context::<'static>::build(&ansi::SPEC, schema, &input, cursor).unwrap();
        let mut candidates = CandidateSet::default();
        complete(&mut ctx, &mut candidates);
        candidates
    }

    #[test]
    fn detect_scope_columns() {
        let sql = "
        WITH cte as (SELECT name as user_name FROM users) 
            SELECT * 
            FROM (SELECT 1, title FROM posts) a, 
                (SELECT 2 as two, users.* FROM public.users) b";

        let c = ansi(sql);
        // let c = ansi("SELECT *, ^ FROM posts");
        panic!("{:?}", c);
    }
}
