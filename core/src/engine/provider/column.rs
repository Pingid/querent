use std::collections::{HashMap, HashSet};

use crate::{
    catalog::{CatalogRead, CatalogResult, schema},
    engine::{ColumnCompletion, Completion, CompletionKind, context},
    token::OpTag,
};

pub fn supports(ctx: &context::Context) -> bool {
    match ctx.clause {
        context::ClauseKind::Select => is_select_position(ctx),
        context::ClauseKind::From => is_from_position(ctx),
        context::ClauseKind::Where => is_where_position(ctx),
        context::ClauseKind::GroupBy => is_group_by_position(ctx),
        context::ClauseKind::OrderBy => is_order_by_position(ctx),
        context::ClauseKind::Using => is_using_position(ctx),
        _ => false,
    }
}

pub async fn complete<C: CatalogRead + ?Sized>(
    ctx: &context::Context,
    catalog: &C,
) -> CatalogResult<Vec<Completion>> {
    // Handle qualified column access (e.g., "users.^")
    if let Some(completions) = complete_qualified_columns(catalog, ctx).await? {
        return Ok(completions);
    }

    // Handle CTEs or subqueries - return their projected columns
    if let Some(completions) = complete_from_derived_table(catalog, ctx).await? {
        return Ok(completions);
    }

    // Gather context information
    let analysis = analyze_context(ctx);
    let base_relations = extract_base_relations(ctx);

    // Determine which tables to query
    let eligible_tables = determine_eligible_tables(
        catalog,
        ctx,
        &base_relations,
        &analysis.selected_relations,
        &analysis.already_selected,
    )
    .await?;

    // Collect columns from all eligible tables
    let mut columns = collect_columns_from_tables(
        catalog,
        &eligible_tables,
        &base_relations,
        analysis.use_qualified,
    )
    .await?;

    // Apply clause-specific filtering
    apply_clause_filters(&mut columns, ctx, &analysis);

    // Convert to completions
    Ok(columns_to_completions(columns, analysis.use_qualified, ctx))
}

// Common commit characters for column completions
const COMMIT_CHARS: [char; 4] = [',', ')', ' ', '\n'];

// ============================================================================
// Utility Functions
// ============================================================================

/// Analysis of the current context for column ContextAnalysis<'a> {
struct ContextAnalysis<'a> {
    already_selected: HashSet<&'a str>,
    use_qualified: bool,
    selected_relations: HashSet<context::RelationId>,
}

/// Analyze context to extract useful information for column completion
fn analyze_context(ctx: &context::Context) -> ContextAnalysis<'_> {
    let already_selected = ctx
        .scope
        .projected
        .iter()
        .map(|c| c.name.as_str())
        .collect();

    let use_qualified = ctx.scope.projected.iter().any(|c| c.qualifier.is_some());

    let selected_relations = ctx
        .scope
        .projected
        .iter()
        .filter_map(|col| match &col.origin {
            context::Origin::BaseColumn { relation, .. } => Some(*relation),
            _ => None,
        })
        .collect();
    ContextAnalysis {
        already_selected,
        use_qualified,
        selected_relations,
    }
}

/// Extract base table relations from the scope
fn extract_base_relations(
    ctx: &context::Context,
) -> Vec<(&context::RelationBinding, &Vec<String>)> {
    ctx.scope
        .relations
        .values()
        .filter_map(|rel| {
            if let context::RelationKind::Base(path) = &rel.kind {
                Some((rel, &path.0))
            } else {
                None
            }
        })
        .collect()
}

/// Handle completions for qualified column access (e.g., "users.^")
async fn complete_qualified_columns<C: CatalogRead + ?Sized>(
    catalog: &C,
    ctx: &context::Context,
) -> CatalogResult<Option<Vec<Completion>>> {
    // If there's a qualifier, we must handle it (return Some) even if empty
    let Some(qualifier) = ctx.cursor.qualifier.as_ref() else {
        return Ok(None);
    };

    // Try to find the relation
    let Some(rel_id) = ctx.scope.relation(qualifier) else {
        // Qualifier exists but doesn't match any relation - return empty
        return Ok(Some(vec![]));
    };

    let Some(rel) = ctx.scope.relations.get(&rel_id) else {
        // Relation ID found but relation doesn't exist - return empty
        return Ok(Some(vec![]));
    };

    match &rel.kind {
        context::RelationKind::Base(path) => {
            let Some((schema, table)) = parse_path_parts(&path.0) else {
                return Ok(Some(vec![]));
            };

            let cols = catalog.list_columns(&table, &schema).await?;

            let source = format_source(&schema, &table);
            Ok(Some(
                cols.into_iter()
                    .map(|c| {
                        let name = c.column_name.clone();
                        make_completion(name, Some(source.clone()), Some(c), ctx)
                    })
                    .collect(),
            ))
        }
        context::RelationKind::Subquery(scope) | context::RelationKind::Cte(scope) => Ok(Some(
            completions_from_projection(catalog, scope, ctx).await?,
        )),
    }
}

/// Handle completions from CTEs or subqueries
async fn complete_from_derived_table<C: CatalogRead + ?Sized>(
    catalog: &C,
    ctx: &context::Context,
) -> CatalogResult<Option<Vec<Completion>>> {
    for rel in ctx.scope.relations.values() {
        if let context::RelationKind::Cte(scope) | context::RelationKind::Subquery(scope) =
            &rel.kind
        {
            return Ok(Some(
                completions_from_projection(catalog, scope, ctx).await?,
            ));
        }
    }
    Ok(None)
}

/// Determine which tables are eligible for column completion
async fn determine_eligible_tables<C: CatalogRead + ?Sized>(
    catalog: &C,
    ctx: &context::Context,
    base_relations: &[(&context::RelationBinding, &Vec<String>)],
    selected_relations: &HashSet<context::RelationId>,
    already_selected: &HashSet<&str>,
) -> CatalogResult<Vec<(String, String)>> {
    if !base_relations.is_empty() {
        // FROM clause exists - use those tables (no catalog lookup needed)
        return Ok(base_relations
            .iter()
            .filter_map(|(_, parts)| parse_path_parts(parts))
            .collect());
    }

    if !selected_relations.is_empty() {
        // Use tables from selected column relations
        return Ok(selected_relations
            .iter()
            .filter_map(|&rel_id| {
                ctx.scope.relations.get(&rel_id).and_then(|rel| {
                    if let context::RelationKind::Base(path) = &rel.kind {
                        parse_path_parts(&path.0)
                    } else {
                        None
                    }
                })
            })
            .collect());
    }

    if !already_selected.is_empty() {
        // Find tables that contain all selected columns
        return find_tables_with_columns(catalog, already_selected).await;
    }

    // No constraints - return all tables
    fetch_all_tables(catalog).await
}

/// Collect columns from eligible tables
async fn collect_columns_from_tables<C: CatalogRead + ?Sized>(
    catalog: &C,
    eligible_tables: &[(String, String)],
    base_relations: &[(&context::RelationBinding, &Vec<String>)],
    _use_qualified: bool,
) -> CatalogResult<Vec<(String, String, String, Option<schema::Column>)>> {
    let mut columns = Vec::new();

    for (schema, table) in eligible_tables {
        let cols = catalog.list_columns(table, &schema).await?;

        let source = format_source(schema, table);
        // Always get qualifier for duplicate detection, even if not used for display yet
        let qualifier = find_table_qualifier(base_relations, schema, table).unwrap_or(table);

        columns.reserve(cols.len());
        for c in cols {
            let name = c.column_name.clone();
            columns.push((name, source.clone(), qualifier.to_string(), Some(c)));
        }
    }

    Ok(columns)
}

/// Apply clause-specific filtering to columns
fn apply_clause_filters(
    columns: &mut Vec<(String, String, String, Option<schema::Column>)>,
    ctx: &context::Context,
    analysis: &ContextAnalysis,
) {
    match ctx.clause {
        context::ClauseKind::Select => {
            columns.retain(|(name, _, _, _)| !analysis.already_selected.contains(name.as_str()));
        }
        context::ClauseKind::GroupBy => {
            filter_for_group_by(columns, ctx);
        }
        context::ClauseKind::OrderBy => {
            filter_for_order_by(columns, ctx);
        }
        context::ClauseKind::Using => {
            filter_for_using(columns, ctx);
        }
        _ => {}
    }
}

/// Convert column tuples to Completion objects
fn columns_to_completions(
    columns: Vec<(String, String, String, Option<schema::Column>)>,
    use_qualified: bool,
    ctx: &context::Context,
) -> Vec<Completion> {
    // Count occurrences of each column name to detect duplicates
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for (name, _, _, _) in &columns {
        *name_counts.entry(name.clone()).or_insert(0) += 1;
    }

    columns
        .into_iter()
        .map(|(name, source, qualifier, column)| {
            // Use qualified name in label if there are duplicate column names
            let has_duplicates = name_counts.get(&name).copied().unwrap_or(0) > 1;

            let (label, insert_text, filter_text) = if use_qualified && !qualifier.is_empty() {
                let qualified = format!("{}.{}", qualifier, name);
                (qualified.clone(), qualified, Some(name.clone()))
            } else if has_duplicates && !qualifier.is_empty() {
                // Show qualified name in label for duplicates
                let qualified = format!("{}.{}", qualifier, name);
                (qualified.clone(), qualified, Some(name.clone()))
            } else {
                (name.clone(), name.clone(), Some(name.clone()))
            };

            Completion {
                label,
                insert_text,
                filter_text,
                kind: CompletionKind::Column(ColumnCompletion {
                    qualifier: Some(source),
                    column,
                }),
                replace: ctx.cursor.replace,
                commit_characters: COMMIT_CHARS.into(),
            }
        })
        .collect()
}

/// Find tables that contain all specified columns
async fn find_tables_with_columns<C: CatalogRead + ?Sized>(
    catalog: &C,
    selected_columns: &HashSet<&str>,
) -> CatalogResult<Vec<(String, String)>> {
    let schemas = catalog.list_schemas().await?;
    let mut eligible = Vec::new();

    for schema in schemas {
        let tables = catalog.list_tables(&schema).await?;
        for table in tables {
            let cols = catalog.list_columns(&table, &schema).await?;
            let col_names: HashSet<_> = cols.iter().map(|c| c.column_name.as_str()).collect();

            if selected_columns.iter().all(|sel| col_names.contains(sel)) {
                eligible.push((schema.clone(), table));
            }
        }
    }

    Ok(eligible)
}

/// Fetch all tables from the catalog
async fn fetch_all_tables<C: CatalogRead + ?Sized>(
    catalog: &C,
) -> CatalogResult<Vec<(String, String)>> {
    let schemas = catalog.list_schemas().await?;
    let mut all_tables = Vec::new();

    for schema in schemas {
        let tables = catalog.list_tables(&schema).await?;
        for table in tables {
            all_tables.push((schema.clone(), table));
        }
    }

    Ok(all_tables)
}

async fn completions_from_projection<C: CatalogRead + ?Sized>(
    catalog: &C,
    scope: &context::Scope,
    ctx: &context::Context,
) -> CatalogResult<Vec<Completion>> {
    let mut out = Vec::new();
    for col in &scope.projected {
        let (source, column_schema) = match &col.origin {
            context::Origin::BaseColumn { relation, name, .. } => {
                if let Some(rel) = scope.relations.get(relation) {
                    if let context::RelationKind::Base(path) = &rel.kind {
                        let source = resolve_source_from_parts(catalog, &path.0).await?;

                        // Try to fetch the actual column from the catalog
                        let parts = &path.0;
                        let (schema, table) = match parts.len() {
                            1 => ("", &parts[0]),
                            2 => (parts[0].as_str(), &parts[1]),
                            _ => ("", &parts[0]),
                        };

                        let cols = catalog.list_columns(table, schema).await?;
                        let column = cols.into_iter().find(|c| &c.column_name == name);

                        (source, column)
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            }
            _ => {
                // For non-base columns (e.g., computed columns), construct a minimal Column
                let column = col.ty.as_ref().map(|ty| schema::Column {
                    column_name: col.name.clone(),
                    data_type: Some(ty.clone()),
                    nullable: true, // Conservative assumption
                    default: None,
                    is_pk: false,
                    generated: false,
                    collation: None,
                    comment: None,
                    ordinal: None,
                });
                (None, column)
            }
        };

        out.push(make_completion(
            col.name.clone(),
            source,
            column_schema,
            ctx,
        ));
    }
    Ok(out)
}

async fn resolve_source_from_parts<C: CatalogRead + ?Sized>(
    catalog: &C,
    parts: &[String],
) -> CatalogResult<Option<String>> {
    let (schema, table) = match parts.len() {
        1 => {
            // Try to resolve schema by scanning, otherwise leave empty
            let schemas = catalog.list_schemas().await?;
            let mut found_schema = String::new();
            for s in &schemas {
                let tables = catalog.list_tables(s).await?;
                if tables.contains(&parts[0]) {
                    found_schema = s.clone();
                    break;
                }
            }
            (found_schema, parts[0].clone())
        }
        2 => (parts[0].clone(), parts[1].clone()),
        _ => return Ok(None),
    };

    Ok(Some(if schema.is_empty() {
        table
    } else {
        format!("{}.{}", schema, table)
    }))
}

/// Filter columns for GROUP BY clause
fn filter_for_group_by(
    columns: &mut Vec<(String, String, String, Option<schema::Column>)>,
    ctx: &context::Context,
) {
    let already_grouped_names: HashSet<_> =
        ctx.scope.grouped.iter().map(|c| c.name.as_str()).collect();

    let grouped_base_columns: HashSet<_> = ctx
        .scope
        .grouped
        .iter()
        .filter_map(|c| match &c.origin {
            context::Origin::BaseColumn { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();

    let projected_base_col_names: HashSet<_> = ctx
        .scope
        .projected
        .iter()
        .filter_map(|c| match &c.origin {
            context::Origin::BaseColumn { .. } => Some(c.name.as_str()),
            _ => None,
        })
        .collect();

    columns.retain(|(name, _, _, _)| {
        let in_projected_base = projected_base_col_names.contains(name.as_str());
        let not_grouped = !already_grouped_names.contains(name.as_str())
            && !grouped_base_columns.contains(name.as_str());
        in_projected_base && not_grouped
    });
}

/// Filter columns for ORDER BY clause
fn filter_for_order_by(
    columns: &mut Vec<(String, String, String, Option<schema::Column>)>,
    ctx: &context::Context,
) {
    let already_ordered_names: HashSet<_> =
        ctx.scope.ordered.iter().map(|c| c.name.as_str()).collect();

    let ordered_base_columns: HashSet<_> = ctx
        .scope
        .ordered
        .iter()
        .filter_map(|c| match &c.origin {
            context::Origin::BaseColumn { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();

    columns.retain(|(name, _, _, _)| {
        !already_ordered_names.contains(name.as_str())
            && !ordered_base_columns.contains(name.as_str())
    });
}

/// Parse path parts into (schema, table)
fn parse_path_parts(parts: &[String]) -> Option<(String, String)> {
    match parts.len() {
        1 => Some((String::new(), parts[0].clone())),
        2 => Some((parts[0].clone(), parts[1].clone())),
        _ => None,
    }
}

/// Format source string from schema and table
fn format_source(schema: &str, table: &str) -> String {
    if schema.is_empty() {
        table.to_string()
    } else {
        format!("{}.{}", schema, table)
    }
}

/// Find the qualifier (alias or table name) for a table
fn find_table_qualifier<'a>(
    base_relations: &[(&'a context::RelationBinding, &Vec<String>)],
    schema: &str,
    table: &str,
) -> Option<&'a str> {
    base_relations
        .iter()
        .find(|(_, parts)| match parts.len() {
            1 => &parts[0] == table,
            2 => &parts[0] == schema && &parts[1] == table,
            _ => false,
        })
        .and_then(|(rel, _)| rel.alias.as_deref())
}

/// Filter columns for USING clause (only common columns)
fn filter_for_using(
    columns: &mut Vec<(String, String, String, Option<schema::Column>)>,
    ctx: &context::Context,
) {
    let column_counts: HashMap<String, usize> =
        columns
            .iter()
            .fold(HashMap::new(), |mut acc, (name, _, _, _)| {
                *acc.entry(name.clone()).or_insert(0) += 1;
                acc
            });

    let table_count = ctx.scope.relations.len();

    columns.retain(|(name, _, _, _)| column_counts.get(name).copied().unwrap_or(0) == table_count);

    // Deduplicate by column name
    let mut seen = HashSet::new();
    columns.retain(|(name, _, _, _)| seen.insert(name.clone()));
}

fn is_select_position(ctx: &context::Context) -> bool {
    match &ctx.cursor.location {
        context::Location::Space(inner) => {
            matches!(
                **inner,
                context::Location::Keyword | context::Location::Comma | context::Location::Dot
            )
        }
        context::Location::Dot => true,
        _ => false,
    }
}

fn is_from_position(ctx: &context::Context) -> bool {
    // Support column completions in FROM clause for:
    // 1. After ON keyword (JOIN ... ON ^)
    // 2. After logical operators in ON conditions (JOIN ... ON a = b AND ^)
    // 3. After dot for qualified names (JOIN ... ON users.^)
    match &ctx.cursor.location {
        context::Location::Space(inner) => match **inner {
            context::Location::Keyword => {
                // Check if the current keyword is ON or if ON is in preceding
                ctx.cursor.current_keyword == Some(crate::token::Keyword::On)
                    || ctx.cursor.preceding.contains(&crate::token::Keyword::On)
            }
            context::Location::Operator(op) => {
                // After logical operators (AND, OR) in ON conditions
                // Also check that we're in a context with ON (JOIN clause)
                matches!(op, OpTag::And | OpTag::Or)
                    && (ctx.cursor.current_keyword == Some(crate::token::Keyword::On)
                        || ctx.cursor.preceding.contains(&crate::token::Keyword::On))
            }
            context::Location::Dot => true,
            _ => false,
        },
        context::Location::Dot => true,
        _ => false,
    }
}

fn is_where_position(ctx: &context::Context) -> bool {
    match &ctx.cursor.location {
        context::Location::Space(inner) => match **inner {
            context::Location::Keyword => true,
            context::Location::Operator(op) => matches!(op, OpTag::And | OpTag::Or),
            _ => false,
        },
        context::Location::Dot => true,
        _ => false,
    }
}

fn is_group_by_position(ctx: &context::Context) -> bool {
    matches!(
        &ctx.cursor.location,
        context::Location::Space(inner) if matches!(**inner, context::Location::Keyword | context::Location::Comma)
    )
}

fn is_order_by_position(ctx: &context::Context) -> bool {
    matches!(
        &ctx.cursor.location,
        context::Location::Space(inner) if matches!(**inner, context::Location::Keyword | context::Location::Comma)
    )
}

fn is_using_position(ctx: &context::Context) -> bool {
    match &ctx.cursor.location {
        context::Location::Paren => true,
        context::Location::Space(inner) => matches!(
            **inner,
            context::Location::Keyword | context::Location::Paren
        ),
        _ => false,
    }
}

fn make_completion(
    name: String,
    source: Option<String>,
    column: Option<schema::Column>,
    ctx: &context::Context,
) -> Completion {
    Completion {
        label: name.clone(),
        insert_text: name.clone(),
        filter_text: Some(name),
        kind: CompletionKind::Column(ColumnCompletion {
            qualifier: source,
            column,
        }),
        replace: ctx.cursor.replace,
        commit_characters: COMMIT_CHARS.into(),
    }
}
