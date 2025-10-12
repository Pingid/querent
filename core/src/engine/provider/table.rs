use crate::{
    catalog::{CatalogRead, CatalogResult},
    engine::{Completion, CompletionKind, TableCompletion, context},
};

pub fn supports(ctx: &context::Context) -> bool {
    // Provide completions in FROM clause
    if !matches!(ctx.clause, context::ClauseKind::From) {
        return false;
    }

    match &ctx.cursor.location {
        // After FROM keyword or after comma (e.g., "FROM users, ^")
        context::Location::Space(inner) => matches!(
            **inner,
            context::Location::Keyword | context::Location::Comma
        ),
        // After schema qualifier dot (e.g., "FROM public.^")
        context::Location::Dot => ctx.cursor.qualifier.is_some(),
        _ => false,
    }
}

pub async fn complete<C: CatalogRead + ?Sized>(
    ctx: &context::Context,
    catalog: &C,
) -> CatalogResult<Vec<Completion>> {
    // If we have a qualifier (e.g., "public.^"), only suggest tables from that schema
    if let Some(qualifier) = &ctx.cursor.qualifier {
        let schema_tables = catalog.list_tables(qualifier).await?;
        let mut completions = Vec::new();

        for name in schema_tables {
            let table = catalog.get_table(&name, qualifier).await?;

            completions.push(Completion {
                label: name.clone(),
                insert_text: name.clone(),
                filter_text: Some(name.clone()),
                kind: CompletionKind::Table(TableCompletion {
                    qualifier: Some(qualifier.clone()),
                    table,
                }),
                replace: ctx.cursor.replace,
                commit_characters: vec![' ', ',', '\n'],
            });
        }

        return Ok(completions);
    }

    // Get all schemas and tables from catalog
    let schemas = catalog.list_schemas().await?;
    let mut tables: Vec<(String, String)> = Vec::new(); // (table_name, schema)

    for schema in &schemas {
        let schema_tables = catalog.list_tables(schema).await?;
        for table in schema_tables {
            tables.push((table, schema.clone()));
        }
    }

    // If after a comma, filter out tables already in FROM
    let tables = if matches!(&ctx.cursor.location, context::Location::Space(inner) if matches!(**inner, context::Location::Comma))
    {
        let existing_tables: Vec<String> = ctx
            .scope
            .relations
            .values()
            .filter_map(|rel| match &rel.kind {
                crate::engine::context::RelationKind::Base(path) => {
                    path.0.last().map(|s| s.to_string())
                }
                _ => None,
            })
            .collect();

        tables
            .into_iter()
            .filter(|(name, _)| !existing_tables.contains(name))
            .collect()
    } else {
        tables
    };

    // Convert to completions
    let mut completions = Vec::new();
    for (name, schema) in tables {
        let table = catalog.get_table(&name, &schema).await?;

        let qualifier = if schema.is_empty() {
            None
        } else {
            Some(schema)
        };

        completions.push(Completion {
            label: name.clone(),
            insert_text: name.clone(),
            filter_text: Some(name.clone()),
            kind: CompletionKind::Table(TableCompletion { qualifier, table }),
            replace: ctx.cursor.replace,
            commit_characters: vec![' ', ',', '\n'],
        });
    }
    Ok(completions)
}
