use std::{future::Future, pin::Pin};

use crate::{
    catalog::CatalogRead,
    dialect::DialectSpec,
    engine::{
        Completion, CompletionKind, TableCompletion,
        context::{ClauseKind, Context, Location},
    },
};

use super::CompletionProvider;

/// Provide table name completions.
pub struct TableProvider;

impl CompletionProvider for TableProvider {
    fn supports(&self, ctx: &Context) -> bool {
        // Provide completions in FROM clause
        if !matches!(ctx.clause, ClauseKind::From) {
            return false;
        }

        match &ctx.cursor.location {
            // After FROM keyword or after comma (e.g., "FROM users, ^")
            Location::Space(inner) => matches!(**inner, Location::Keyword | Location::Comma),
            // After schema qualifier dot (e.g., "FROM public.^")
            Location::Dot => ctx.cursor.qualifier.is_some(),
            _ => false,
        }
    }

    fn complete<'a>(
        &'a self,
        catalog: &'a (dyn CatalogRead + Send + Sync),
        _spec: &'a DialectSpec,
        ctx: &'a Context,
    ) -> Pin<Box<dyn Future<Output = Vec<Completion>> + Send + 'a>> {
        Box::pin(async move {
            // If we have a qualifier (e.g., "public.^"), only suggest tables from that schema
            if let Some(qualifier) = &ctx.cursor.qualifier {
                let schema_tables = catalog.list_tables(qualifier).await;
                let mut completions = Vec::new();

                for name in schema_tables {
                    let table = catalog.get_table(&name, qualifier).await;

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

                return completions;
            }

            // Get all schemas and tables from catalog
            let schemas = catalog.list_schemas().await;
            let mut tables: Vec<(String, String)> = Vec::new(); // (table_name, schema)

            for schema in &schemas {
                let schema_tables = catalog.list_tables(schema).await;
                for table in schema_tables {
                    tables.push((table, schema.clone()));
                }
            }

            // If after a comma, filter out tables already in FROM
            let tables = if matches!(&ctx.cursor.location, Location::Space(inner) if matches!(**inner, Location::Comma))
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
                let table = catalog.get_table(&name, &schema).await;

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
            completions
        })
    }
}
