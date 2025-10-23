use crate::complete::Context;
use crate::complete::context::{Qualifier, ResolvedColumn, ResolvedColumnSource, ScopeResolve};
use crate::schema;

#[derive(Debug, Clone, PartialEq)]
pub struct AvailableColumn<'txt, 'schema> {
    name: String,
    score: i8,
    source_alias: Option<&'txt str>,
    qualifier: Qualifier<'txt>,
    source: AvailableColumnSource<'schema>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AvailableColumnSource<'schema> {
    Schema(&'schema schema::Column),
    Unresolved { ty: Option<schema::DataType> },
}

impl<'txt, 'schema> From<ResolvedColumn<'txt, 'schema>> for AvailableColumn<'txt, 'schema> {
    fn from(col: ResolvedColumn<'txt, 'schema>) -> Self {
        match col.source {
            ResolvedColumnSource::Schema(c) => Self {
                name: col.name,
                score: 0,
                source_alias: col.source_alias,
                qualifier: col.qualifier,
                source: AvailableColumnSource::Schema(c),
            },
            ResolvedColumnSource::Literal { ty } => Self {
                name: col.name,
                score: 0,
                source_alias: col.source_alias,
                qualifier: col.qualifier,
                source: AvailableColumnSource::Unresolved {
                    ty: Some(ty.clone()),
                },
            },
            ResolvedColumnSource::Unresolved(qualifier) => Self {
                name: col.name,
                score: 0,
                source_alias: col.source_alias,
                qualifier: qualifier,
                source: AvailableColumnSource::Unresolved { ty: None },
            },
        }
    }
}

impl<'txt, 'schema> From<&'schema schema::Column> for AvailableColumn<'txt, 'schema> {
    fn from(col: &'schema schema::Column) -> Self {
        Self {
            score: 0,
            name: col.column_name.clone(),
            source_alias: None,
            qualifier: Qualifier::default(),
            source: AvailableColumnSource::Schema(col),
        }
    }
}

impl<'txt, 'schema> AvailableColumn<'txt, 'schema> {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn score(&self) -> i8 {
        self.score
    }

    pub fn update_score(&mut self, by: i8) {
        self.score += by;
    }

    pub fn schema_name(&self) -> Option<&'txt str> {
        self.qualifier.schema
    }

    /// Alias or else qualifier or else schema table name
    pub fn source_name(&self) -> Option<String> {
        self.source_alias
            .map(|x| x.to_string())
            .or_else(|| self.qualifier.table.map(|x| x.to_string()))
            .or_else(|| match &self.source {
                AvailableColumnSource::Schema(c) => c.table_name.clone(),
                AvailableColumnSource::Unresolved { .. } => None,
            })
    }

    /// Check if the column qualifier matches the other qualifier
    pub fn matches_qualifier(&self, qualifier: &Option<Vec<&'txt str>>) -> bool {
        let Some(qualifier) = qualifier else {
            return true;
        };
        match qualifier.len() {
            0 => true,
            1 => Some(qualifier[0].to_string()) == self.source_name(),
            _ => {
                Some(qualifier[0].to_string()) == self.source_name()
                    && Some(qualifier[1]) == self.schema_name()
            }
        }
    }

    /// Get the completion detail for the column
    pub fn detail(&self) -> String {
        match &self.source {
            AvailableColumnSource::Schema(c) => {
                detail(&Qualifier::from(*c), &c.column_name, &Some(c.data_type))
            }
            AvailableColumnSource::Unresolved { ty } => detail(&self.qualifier, &self.name, &ty),
        }
    }
}

/// Get formatted detail for the column
fn detail(qualifier: &Qualifier, name: &str, ty: &Option<schema::DataType>) -> String {
    let q = qualifier.to_string();
    let q = match q.is_empty() {
        false => format!("{}.{}", q, name),
        true => name.to_string(),
    };
    match ty {
        Some(ty) => format!("{} ({})", q, ty.to_string()),
        None => q,
    }
}

pub fn get_scope_available_columns<'txt>(ctx: &Context<'txt>) -> Vec<AvailableColumn<'txt, 'txt>> {
    let mut cols = Vec::new();

    // Find all exposed columns from CTE's, FROM tables/subqueries, etc.
    let available = ctx.scope.resolve_available_columns(ctx.schema);

    // If no columns are available, add all columns from the schema.
    if available.is_empty() {
        for col in ctx.schema.get_columns().iter() {
            cols.push(AvailableColumn::from(col));
        }
    }

    // Add all available columns to the list.
    for col in available {
        cols.push(AvailableColumn::from(col));
    }

    // Filter out columns that don't match the qualifier.
    cols.retain(|col| col.matches_qualifier(&ctx.cursor.qualifier));

    cols
}

pub fn get_qualified_name(col: &AvailableColumn) -> Option<String> {
    let Some(table_name) = col.source_name() else {
        return None;
    };
    Some(format!("{}.{}", table_name, col.name()))
}
