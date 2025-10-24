use std::collections::HashMap;

use super::scope::*;
use crate::schema;

#[derive(Debug, Clone)]
pub struct ResolvedScope<'a> {
    scope: Scope<'a>,
    schema: &'a schema::Cache,
    projected: Option<Vec<ResolvedColumn<'a>>>,
    available: Option<Vec<ResolvedColumn<'a>>>,
    cte_names: Option<Vec<&'a str>>,
}

impl<'a> ResolvedScope<'a> {
    pub fn new(scope: Scope<'a>, schema: &'a schema::Cache) -> Self {
        Self {
            scope,
            schema,
            projected: None,
            available: None,
            cte_names: None,
        }
    }
}

impl<'a> ResolvedScope<'a> {
    pub fn relations(&self) -> &HashMap<RelationId, RelationBinding<'a>> {
        &self.scope.relations
    }

    pub fn cte_names(&mut self) -> &Vec<&'a str> {
        if self.cte_names.is_none() {
            self.cte_names = Some(get_cte_names(&self.scope));
        }
        &self.cte_names.as_ref().unwrap()
    }

    pub fn projected(&mut self) -> &Vec<ResolvedColumn<'a>> {
        if self.projected.is_none() {
            self.projected = Some(resolve_projected_columns(&self.scope, self.schema));
        }
        &self.projected.as_ref().unwrap()
    }

    pub fn available_columns(&mut self) -> &Vec<ResolvedColumn<'a>> {
        if self.available.is_none() {
            self.available = Some(resolve_available_columns(&self.scope, self.schema));
        }
        &self.available.as_ref().unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct ResolvedColumn<'a> {
    pub name: String,
    pub source: ResolvedColumnSource<'a>,
    pub source_alias: Option<&'a str>,
    pub qualifier: Qualifier<'a>,
}

impl<'a> ResolvedColumn<'a> {
    pub fn matches_source_name(&self, source_name: &String) -> bool {
        self.source_name().map_or(false, |x| x == source_name)
    }

    pub fn source_name(&self) -> Option<&str> {
        self.source_alias.or_else(|| match &self.source {
            ResolvedColumnSource::Schema(c) => c.table_name.as_ref().map(|s| s.as_str()),
            ResolvedColumnSource::Literal { .. } => None,
            ResolvedColumnSource::Unresolved(qualifier) => qualifier.table,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum ResolvedColumnSource<'a> {
    Schema(&'a schema::Column),
    Literal { ty: schema::DataType },
    Unresolved(Qualifier<'a>),
}

impl<'a> ResolvedColumnSource<'a> {
    pub fn table_name(&self) -> Option<&str> {
        match self {
            ResolvedColumnSource::Schema(c) => c.table_name.as_ref().map(|s| s.as_str()),
            ResolvedColumnSource::Literal { .. } => None,
            ResolvedColumnSource::Unresolved(qualifier) => qualifier.table,
        }
    }
}

fn get_cte_names<'a>(scope: &Scope<'a>) -> Vec<&'a str> {
    scope
        .relations
        .values()
        .filter_map(|r| match &r.kind {
            RelationKind::Cte(_) => r.alias,
            _ => None,
        })
        .collect::<Vec<_>>()
}

fn resolve_projected_columns<'a>(
    scope: &Scope<'a>, schema: &'a schema::Cache,
) -> Vec<ResolvedColumn<'a>> {
    let mut cols = Vec::new();
    for column in &scope.projected {
        cols.extend(get_resolved_columns_for_bound_column(scope, schema, column));
    }
    cols
}

fn resolve_available_columns<'a>(
    scope: &Scope<'a>, schema: &'a schema::Cache,
) -> Vec<ResolvedColumn<'a>> {
    let mut cols = Vec::new();
    for relation in scope.relations.values() {
        match &relation.kind {
            RelationKind::Base(path) => {
                let qualifier = Qualifier::from(&path.0);
                cols.extend(find_columns_in_schema(schema, &qualifier, None).map(|x| {
                    ResolvedColumn {
                        name: x.column_name.clone(),
                        source: ResolvedColumnSource::Schema(x),
                        source_alias: relation.alias,
                        qualifier: qualifier.clone(),
                    }
                }))
            }
            RelationKind::Cte(scope) => cols.extend(
                resolve_projected_columns(scope, schema)
                    .into_iter()
                    .map(|mut x| {
                        x.source_alias = relation.alias;
                        x
                    }),
            ),
            RelationKind::Subquery(scope) => {
                let projected = resolve_projected_columns(scope, schema)
                    .into_iter()
                    .map(|mut x| {
                        x.source_alias = relation.alias;
                        x
                    })
                    .collect::<Vec<_>>();

                cols.extend(projected);
            }
        }
    }
    cols
}

fn get_resolved_columns_for_bound_column<'a>(
    scope: &Scope<'a>, schema: &'a schema::Cache, column: &BoundColumn<'a>,
) -> Vec<ResolvedColumn<'a>> {
    let mut cols = Vec::new();
    match &column.origin {
        Origin::BaseColumn { relation, name } => {
            if let Some(relation) = scope.relations.get(relation) {
                match &relation.kind {
                    RelationKind::Base(path) => {
                        let qualifier = Qualifier::from(&path.0);
                        match find_column_in_schema(schema, &qualifier, Some(name)) {
                            Some(c) => {
                                cols.push(ResolvedColumn {
                                    name: column.name.to_string(),
                                    source: ResolvedColumnSource::Schema(c),
                                    source_alias: relation.alias,
                                    qualifier: qualifier.clone(),
                                });
                            }
                            None => cols.push(ResolvedColumn {
                                name: column.name.to_string(),
                                source: ResolvedColumnSource::Unresolved(qualifier.clone()),
                                source_alias: relation.alias,
                                qualifier: qualifier.clone(),
                            }),
                        }
                    }
                    RelationKind::Cte(_) => {}
                    RelationKind::Subquery(scope) => {
                        let projected = resolve_projected_columns(scope, schema);

                        let found: Vec<ResolvedColumn<'a>> = projected
                            .into_iter()
                            .filter(|c| c.name == *name)
                            .map(|mut x| {
                                x.source_alias = relation.alias;
                                x.qualifier = column.qualifier.clone();
                                x
                            })
                            .collect::<Vec<_>>();

                        cols.extend(found);
                    }
                }
            }
        }
        Origin::UnresolvedIdent(path) => {
            let qualifier = column.qualifier.clone();
            let colum_name = path.0.last().copied();
            let found = find_columns_in_schema(schema, &qualifier, colum_name).collect::<Vec<_>>();

            if found.is_empty() {
                cols.push(ResolvedColumn {
                    name: column.name.to_string(),
                    source: ResolvedColumnSource::Unresolved(qualifier.clone()),
                    source_alias: None,
                    qualifier: qualifier.clone(),
                });
            } else {
                found.iter().for_each(|c| {
                    cols.push(ResolvedColumn {
                        name: c.column_name.clone(),
                        source: ResolvedColumnSource::Schema(c),
                        source_alias: None,
                        qualifier: qualifier.clone(),
                    })
                });
            }
        }
        Origin::Star { relation } => {
            if let Some(relation) = relation.and_then(|r| scope.relations.get(&r)) {
                match &relation.kind {
                    RelationKind::Base(path) => {
                        let qualifier: Qualifier = Qualifier::from(&path.0);
                        cols.extend(find_columns_in_schema(schema, &qualifier, None).map(|c| {
                            ResolvedColumn {
                                name: c.column_name.clone(),
                                source: ResolvedColumnSource::Schema(c),
                                source_alias: None,
                                qualifier: Qualifier::default(),
                            }
                        }));
                    }
                    RelationKind::Cte(scope) => {
                        let projected = resolve_projected_columns(scope, schema);
                        cols.extend(projected.into_iter().map(|mut x| {
                            x.source_alias = relation.alias;
                            // x.qualifier = Qualifier::default();
                            x
                        }))
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
                name: column.name.to_string(),
                source: ResolvedColumnSource::Literal { ty },
                source_alias: None,
                qualifier: Qualifier::default(),
            });
        }
    };
    cols
}

fn find_columns_in_schema<'a>(
    schema: &'a schema::Cache, path: &Qualifier, name: Option<&str>,
) -> impl Iterator<Item = &'a schema::Column> {
    schema
        .get_columns()
        .iter()
        .filter(move |c| matches_path(c, path, name))
}

fn find_column_in_schema<'a>(
    schema: &'a schema::Cache, path: &Qualifier, name: Option<&str>,
) -> Option<&'a schema::Column> {
    schema
        .get_columns()
        .iter()
        .find(move |c| matches_path(c, path, name))
}

fn matches_path(c: &schema::Column, path: &Qualifier, name: Option<&str>) -> bool {
    if let Some(t) = path.table
        && c.table_name.as_ref().map_or(true, |c| c.as_str() != t)
    {
        return false;
    }
    if let Some(t) = path.schema
        && c.schema_name.as_ref().map_or(true, |c| c.as_str() != t)
    {
        return false;
    }
    if let Some(n) = name
        && c.column_name != n
    {
        return false;
    }
    true
}
