use super::graph::*;
use crate::complete::context::ContextBuildParams;
use crate::schema;

#[derive(Debug, Clone)]
pub struct Scope<'a> {
    schema: &'a schema::Cache,
    relations: ScopeGraph<'a>,
    projected: Option<Vec<ResolvedColumn<'a>>>,
    available: Option<Vec<ResolvedColumn<'a>>>,
    cte_names: Option<Vec<&'a str>>,
}

impl<'a> From<&ContextBuildParams<'a>> for Scope<'a> {
    fn from(params: &ContextBuildParams<'a>) -> Self {
        let graph = ScopeGraph::from(params);
        Scope {
            schema: params.schema,
            relations: graph,
            projected: None,
            available: None,
            cte_names: None,
        }
    }
}

impl<'a> Scope<'a> {
    /// Returns an iterator over the bindings in the scope.
    pub fn bindings(&mut self) -> impl Iterator<Item = &RelationBinding<'a>> {
        self.relations.bindings.values()
    }

    /// Returns a vector of the names of the CTEs in the scope.
    pub fn ctes(&mut self) -> &Vec<&'a str> {
        self.cte_names
            .get_or_insert_with(|| get_cte_names(&self.relations))
    }

    /// Returns a vector column declared in the SELECT list.
    pub fn projected(&mut self) -> &Vec<ResolvedColumn<'a>> {
        self.projected.get_or_insert_with(|| {
            resolve_projected_columns(&self.relations, self.schema).collect()
        })
    }

    /// Returns all columns that can be referenced at the current cursor
    /// position.
    ///
    /// This includes columns from:
    /// - Base tables referenced in FROM/JOIN clauses
    /// - CTEs (Common Table Expressions) visible at this position
    /// - Subqueries with their projected columns
    pub fn available_columns(&mut self) -> &Vec<ResolvedColumn<'a>> {
        self.available
            .get_or_insert_with(|| resolve_available_columns(&self.relations, self.schema))
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
            ResolvedColumnSource::Cte => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum ResolvedColumnSource<'a> {
    Cte,
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
            ResolvedColumnSource::Cte => None,
        }
    }
}

fn get_cte_names<'a>(relations: &ScopeGraph<'a>) -> Vec<&'a str> {
    relations
        .bindings
        .values()
        .filter_map(|r| match &r.kind {
            BindingKind::Cte(_) => r.alias,
            _ => None,
        })
        .collect::<Vec<_>>()
}

fn resolve_projected_columns<'a>(
    relations: &ScopeGraph<'a>, schema: &'a schema::Cache,
) -> impl Iterator<Item = ResolvedColumn<'a>> {
    relations
        .projected
        .iter()
        .flat_map(move |p| get_resolved_columns_for_bound_column(relations, schema, p))
}

fn resolve_available_columns<'a>(
    relations: &ScopeGraph<'a>, schema: &'a schema::Cache,
) -> Vec<ResolvedColumn<'a>> {
    let mut cols = Vec::new();
    for relation in relations.bindings.values() {
        match &relation.kind {
            BindingKind::Base(path) => {
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
            BindingKind::Cte(scope) => {
                cols.extend(resolve_projected_columns(scope, schema).map(|mut x| {
                    x.source_alias = relation.alias;
                    x
                }))
            }
            BindingKind::Subquery(scope) => {
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
    relations: &ScopeGraph<'a>, schema: &'a schema::Cache, column: &BoundColumn<'a>,
) -> Vec<ResolvedColumn<'a>> {
    let mut cols = Vec::new();
    match &column.origin {
        Origin::BaseColumn { relation, name } => {
            if let Some(relation) = relations.bindings.get(relation) {
                match &relation.kind {
                    BindingKind::Base(path) => {
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
                    BindingKind::Cte(scope) => {
                        let matched =
                            resolve_projected_columns(scope, schema).find(|c| c.name == *name);
                        if let Some(matched) = matched {
                            cols.push(ResolvedColumn {
                                name: matched.name.to_string(),
                                source: ResolvedColumnSource::Cte,
                                source_alias: relation.alias,
                                qualifier: column.qualifier.clone(),
                            });
                        }
                    }
                    BindingKind::Subquery(scope) => {
                        let found: Vec<ResolvedColumn<'a>> =
                            resolve_projected_columns(scope, schema)
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
            let column_name = path.0.last().copied();
            let found = find_columns_in_schema(schema, &qualifier, column_name).collect::<Vec<_>>();

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
            if let Some(relation) = relation.and_then(|r| relations.bindings.get(&r)) {
                match &relation.kind {
                    BindingKind::Base(path) => {
                        let qualifier: Qualifier = Qualifier::from(&path.0);
                        cols.extend(find_columns_in_schema(schema, &qualifier, None).map(|c| {
                            ResolvedColumn {
                                name: c.column_name.clone(),
                                source: ResolvedColumnSource::Schema(c),
                                source_alias: None,
                                qualifier: qualifier.clone(),
                            }
                        }));
                    }
                    BindingKind::Cte(scope) => {
                        let projected = resolve_projected_columns(scope, schema);
                        cols.extend(projected.map(|mut x| {
                            x.source_alias = relation.alias;
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
        Origin::FunctionCall { .. } => {
            // cols.push(ResolvedColumn {
            //     name: column.name.to_string(),
            //     source: ResolvedColumnSource::FunctionCall(name),
            //     source_alias: None,
            //     qualifier: Qualifier::default(),
            // });
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
