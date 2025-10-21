use super::{BoundColumn, Literal, Origin, Qualifier, RelationKind, Scope};
use crate::schema;

pub trait ScopeResolve {
    fn resolve_projected_columns<'a>(&self, schema: &'a schema::Cache) -> Vec<ResolvedColumn<'a>>;
    fn resolve_available_columns<'a>(&self, schema: &'a schema::Cache) -> Vec<ResolvedColumn<'a>>;
    fn get_cte_names(&self) -> Vec<String>;
}

impl ScopeResolve for Scope {
    fn get_cte_names(&self) -> Vec<String> {
        self.relations
            .values()
            .filter_map(|r| match &r.kind {
                RelationKind::Cte(_) => r.alias.clone(),
                _ => None,
            })
            .collect::<Vec<_>>()
    }
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
                    let qualifier = Qualifier::from(&path.0);
                    cols.extend(find_columns_in_schema(schema, &qualifier, None).map(|x| {
                        ResolvedColumn {
                            name: x.column_name.clone(),
                            source: ResolvedColumnSource::Schema(x),
                            source_alias: relation.alias.clone(),
                            qualifier: qualifier.clone(),
                        }
                    }))
                }
                RelationKind::Cte(scope) => cols.extend(
                    scope
                        .resolve_projected_columns(schema)
                        .into_iter()
                        .map(|mut x| {
                            x.source_alias = relation.alias.clone();
                            x
                        }),
                ),
                RelationKind::Subquery(scope) => {
                    let projected = scope
                        .resolve_projected_columns(schema)
                        .into_iter()
                        .map(|mut x| {
                            x.source_alias = relation.alias.clone();
                            x
                        })
                        .collect::<Vec<_>>();

                    cols.extend(projected);
                }
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
                        let qualifier = Qualifier::from(&path.0);
                        match find_column_in_schema(schema, &qualifier, Some(&name)) {
                            Some(c) => {
                                cols.push(ResolvedColumn {
                                    name: column.name.clone(),
                                    source: ResolvedColumnSource::Schema(c),
                                    source_alias: relation.alias.clone(),
                                    qualifier: qualifier.clone(),
                                });
                            }
                            None => cols.push(ResolvedColumn {
                                name: column.name.clone(),
                                source: ResolvedColumnSource::Unresolved(qualifier.clone()),
                                source_alias: relation.alias.clone(),
                                qualifier: qualifier.clone(),
                            }),
                        }
                    }
                    RelationKind::Cte(_) => {}
                    RelationKind::Subquery(scope) => {
                        let projected = scope.resolve_projected_columns(schema);

                        let found: Vec<ResolvedColumn<'_>> = projected
                            .into_iter()
                            .filter(|c| c.name == *name)
                            .map(|mut x| {
                                x.source_alias = relation.alias.clone();
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
            let colum_name = path.0.last();
            let found = find_columns_in_schema(schema, &qualifier, colum_name).collect::<Vec<_>>();

            if found.is_empty() {
                cols.push(ResolvedColumn {
                    name: column.name.clone(),
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
                        let projected = scope.resolve_projected_columns(schema);
                        cols.extend(projected.into_iter().map(|mut x| {
                            x.source_alias = relation.alias.clone();
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
                name: column.name.clone(),
                source: ResolvedColumnSource::Literal { ty },
                source_alias: None,
                qualifier: Qualifier::default(),
            });
        }
    };
    cols
}

fn find_columns_in_schema<'a>(
    schema: &'a schema::Cache,
    path: &Qualifier,
    name: Option<&String>,
) -> impl Iterator<Item = &'a schema::Column> {
    schema
        .get_columns()
        .iter()
        .filter(move |c| matches_path(c, path, name))
}

fn find_column_in_schema<'a>(
    schema: &'a schema::Cache,
    path: &Qualifier,
    name: Option<&String>,
) -> Option<&'a schema::Column> {
    schema
        .get_columns()
        .iter()
        .find(move |c| matches_path(c, path, name))
}

fn matches_path(c: &schema::Column, path: &Qualifier, name: Option<&String>) -> bool {
    if let Some(t) = path.table.as_ref()
        && c.table_name.as_ref().map_or(true, |c| c != t)
    {
        return false;
    }
    if let Some(t) = path.schema.as_ref()
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

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct ResolvedColumn<'a> {
    pub name: String,
    pub source: ResolvedColumnSource<'a>,
    pub source_alias: Option<String>,
    pub qualifier: Qualifier,
}

impl ResolvedColumn<'_> {
    pub fn matches_source_name(&self, source_name: &String) -> bool {
        if self
            .source_alias
            .as_ref()
            .map_or(false, |x| x == source_name)
        {
            return true;
        }
        match &self.source {
            ResolvedColumnSource::Schema(c) => c.table_name.as_ref() == Some(source_name),
            ResolvedColumnSource::Literal { .. } => false,
            ResolvedColumnSource::Unresolved(qualifier) => {
                qualifier.table.as_ref() == Some(source_name)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum ResolvedColumnSource<'a> {
    Schema(&'a schema::Column),
    Literal { ty: schema::DataType },
    Unresolved(Qualifier),
}

impl ResolvedColumnSource<'_> {
    pub fn table_name(&self) -> Option<&String> {
        match self {
            ResolvedColumnSource::Schema(c) => c.table_name.as_ref(),
            ResolvedColumnSource::Literal { .. } => None,
            ResolvedColumnSource::Unresolved(qualifier) => qualifier.table.as_ref(),
        }
    }
}
