use crate::complete::{CompletionBuilder, CompletionKind, PossibleCompletion, TableCompletion};
use crate::schema;

use super::super::context;

pub fn complete(ctx: &context::Context, builder: &mut CompletionBuilder) {
    if !supports(ctx) {
        return;
    }

    // If we have a qualifier (e.g., "public.^"), only suggest tables from that schema
    if let Some(qualifier) = &ctx.cursor.qualifier {
        let schema_tables = list_tables(ctx.schema, qualifier);

        for name in schema_tables {
            let table = get_table(ctx.schema, &name, qualifier);

            builder.add(PossibleCompletion {
                label: name.clone(),
                insert_text: name.clone(),
                filter_text: Some(name.clone()),
                kind: CompletionKind::Table(TableCompletion {
                    qualifier: Some(qualifier.clone()),
                    table,
                }),
                commit_characters: vec![' ', ',', '\n'],
                score: 0,
            });
        }
    }

    // Get all schemas and tables from cache
    let schemas = list_schemas(ctx.schema);
    let mut tables: Vec<(String, String)> = Vec::new(); // (table_name, schema)

    for schema in &schemas {
        let schema_tables = list_tables(ctx.schema, schema);
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
                crate::complete::context::RelationKind::Base(path) => {
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

    for (name, schema) in tables {
        let table = get_table(ctx.schema, &name, &schema);

        let qualifier = if schema.is_empty() {
            None
        } else {
            Some(schema)
        };

        builder.add(PossibleCompletion {
            label: name.clone(),
            insert_text: name.clone(),
            filter_text: Some(name.clone()),
            kind: CompletionKind::Table(TableCompletion { qualifier, table }),
            commit_characters: vec![' ', ',', '\n'],
            score: 0,
        });
    }
}

fn supports(ctx: &context::Context) -> bool {
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

// ============================================================================
// Cache Helper Functions
// ============================================================================

/// List all schemas from the cache
fn list_schemas(cache: &schema::Cache) -> Vec<String> {
    let mut schemas: Vec<String> = cache
        .get_tables()
        .iter()
        .filter_map(|t| t.schema_name.clone())
        .collect();
    schemas.sort();
    schemas.dedup();
    schemas
}

/// List all tables in a schema from the cache
fn list_tables(cache: &schema::Cache, schema: &str) -> Vec<String> {
    if schema.is_empty() {
        // If schema is empty, return all tables
        cache
            .get_tables()
            .iter()
            .map(|t| t.table_name.clone())
            .collect()
    } else {
        cache
            .get_tables()
            .iter()
            .filter(|t| t.schema_name.as_deref() == Some(schema))
            .map(|t| t.table_name.clone())
            .collect()
    }
}

/// Get a specific table from the cache
fn get_table(cache: &schema::Cache, table: &str, schema: &str) -> Option<schema::Table> {
    if schema.is_empty() {
        // If schema is empty, search all schemas for the table
        cache
            .get_tables()
            .iter()
            .find(|t| t.table_name == table)
            .cloned()
    } else {
        cache
            .get_tables()
            .iter()
            .find(|t| t.table_name == table && t.schema_name.as_deref() == Some(schema))
            .cloned()
    }
}
