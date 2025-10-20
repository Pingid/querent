use super::{BoundColumn, Literal, NamePath, Origin, RelationKind, Scope};
use crate::schema;

pub trait ScopeResolve {
    fn resolve_projected_columns<'a>(&self, schema: &'a schema::Cache) -> Vec<ResolvedColumn<'a>>;
    fn resolve_available_columns<'a>(&self, schema: &'a schema::Cache) -> Vec<ResolvedColumn<'a>>;
}

impl ScopeResolve for Scope {
    fn resolve_projected_columns<'a>(&self, schema: &'a schema::Cache) -> Vec<ResolvedColumn<'a>> {
        let mut cols = Vec::new();
        for column in &self.projected {
            cols.extend(get_resolved_columns_for_bound_column(self, schema, column));
        }
        cols
    }

    fn resolve_available_columns<'a>(&self, schema: &'a schema::Cache) -> Vec<ResolvedColumn<'a>> {
        let mut cols = Vec::new();
        for relation in self.relations.values() {
            match &relation.kind {
                RelationKind::Base(path) => {
                    cols.extend(find_column_in_schema(schema, &path, None).map(|x| {
                        ResolvedColumn {
                            name: x.column_name.clone(),
                            source: ResolvedColumnSource::Schema(x),
                            source_alias: relation.alias.clone(),
                        }
                    }))
                }
                RelationKind::Cte(scope) => cols.extend(scope.resolve_projected_columns(schema)),
                RelationKind::Subquery(scope) => cols.extend(
                    scope
                        .resolve_projected_columns(schema)
                        .into_iter()
                        .map(|mut x| {
                            x.source_alias = relation.alias.clone();
                            x
                        }),
                ),
            }
        }
        cols
    }
}

fn get_resolved_columns_for_bound_column<'a>(
    scope: &Scope,
    schema: &'a schema::Cache,
    column: &BoundColumn,
) -> Vec<ResolvedColumn<'a>> {
    let mut cols = Vec::new();
    match &column.origin {
        Origin::BaseColumn { relation, name } => {
            if let Some(relation) = scope.relations.get(relation) {
                match &relation.kind {
                    RelationKind::Base(path) => {
                        match find_column_in_schema(schema, &path, Some(&name)) {
                            Some(c) => {
                                cols.push(ResolvedColumn {
                                    name: column.name.clone(),
                                    source: ResolvedColumnSource::Schema(c),
                                    source_alias: relation.alias.clone(),
                                });
                            }
                            None => cols.push(ResolvedColumn {
                                name: column.name.clone(),
                                source: ResolvedColumnSource::Unresolved(path.clone()),
                                source_alias: relation.alias.clone(),
                            }),
                        }
                    }
                    RelationKind::Cte(_) => {}
                    RelationKind::Subquery(scope) => {
                        let projected = scope.resolve_projected_columns(schema);
                        let found =
                            projected
                                .into_iter()
                                .filter(|c| c.name == *name)
                                .map(|mut x| {
                                    x.source_alias = relation.alias.clone();
                                    x
                                });
                        cols.extend(found);
                    }
                }
            }
        }
        Origin::UnresolvedIdent(path) => cols.push(ResolvedColumn {
            name: column.name.clone(),
            source: ResolvedColumnSource::Unresolved(path.clone()),
            source_alias: None,
        }),
        Origin::Star { relation } => {
            if let Some(relation) = relation.and_then(|r| scope.relations.get(&r)) {
                match &relation.kind {
                    RelationKind::Base(path) => {
                        cols.extend(find_columns_in_schema(schema, &path, None).map(|c| {
                            ResolvedColumn {
                                name: c.column_name.clone(),
                                source: ResolvedColumnSource::Schema(c),
                                source_alias: None,
                            }
                        }));
                    }
                    _ => {}
                };
            }
        }
        Origin::Constant(literal) => {
            let ty = match literal {
                Literal::Number => schema::DataType::Integer,
                Literal::Float => schema::DataType::Float,
                Literal::String => schema::DataType::Text,
                Literal::Boolean => schema::DataType::Boolean,
                Literal::Null => schema::DataType::Null,
            };
            cols.push(ResolvedColumn {
                name: column.name.clone(),
                source: ResolvedColumnSource::Literal { ty },
                source_alias: None,
            });
        }
    };
    cols
}

fn find_columns_in_schema<'a>(
    schema: &'a schema::Cache,
    path: &NamePath,
    name: Option<&str>,
) -> impl Iterator<Item = &'a schema::Column> {
    schema
        .get_columns()
        .iter()
        .filter(move |c| matches_path(c, path, name))
}

fn find_column_in_schema<'a>(
    schema: &'a schema::Cache,
    path: &NamePath,
    name: Option<&str>,
) -> Option<&'a schema::Column> {
    schema
        .get_columns()
        .iter()
        .find(move |c| matches_path(c, path, name))
}

fn matches_path(c: &schema::Column, path: &NamePath, name: Option<&str>) -> bool {
    if let Some(t) = path.table_name()
        && c.table_name.as_ref().map_or(true, |c| c != t)
    {
        return false;
    }
    if let Some(t) = path.schema_name()
        && c.schema_name.as_ref().map_or(true, |c| c != t)
    {
        return false;
    }
    if let Some(n) = name
        && c.column_name != *n
    {
        return false;
    }
    true
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedColumn<'a> {
    pub name: String,
    pub source: ResolvedColumnSource<'a>,
    pub source_alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedColumnSource<'a> {
    Schema(&'a schema::Column),
    Literal { ty: schema::DataType },
    Unresolved(NamePath),
}
