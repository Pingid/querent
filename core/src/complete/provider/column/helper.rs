use crate::complete::Context;
use crate::complete::context::{Qualifier, ResolvedColumn, ResolvedColumnSource, ScopeResolve};
use crate::schema;

#[derive(Debug, Clone, PartialEq)]
pub struct AvailableColumn {
    name: String,
    score: i8,
    source_alias: Option<String>,
    qualifier: Qualifier,
    source: AvailableColumnSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AvailableColumnSource {
    Schema(schema::Column),
    Unresolved { ty: Option<schema::DataType> },
}

impl From<ResolvedColumn<'_>> for AvailableColumn {
    fn from(col: ResolvedColumn<'_>) -> Self {
        match col.source {
            ResolvedColumnSource::Schema(c) => Self {
                name: col.name,
                score: 0,
                source_alias: col.source_alias,
                qualifier: col.qualifier,
                source: AvailableColumnSource::Schema(c.clone()),
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

impl From<schema::Column> for AvailableColumn {
    fn from(col: schema::Column) -> Self {
        Self {
            score: 0,
            name: col.column_name.clone(),
            source_alias: None,
            qualifier: Qualifier::default(),
            source: AvailableColumnSource::Schema(col.clone()),
        }
    }
}

impl AvailableColumn {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn score(&self) -> i8 {
        self.score
    }

    pub fn update_score(&mut self, by: i8) {
        self.score += by;
    }

    pub fn schema_name(&self) -> Option<&String> {
        self.qualifier.schema.as_ref()
    }

    /// Alias or else qualifier or else schema table name
    pub fn source_name(&self) -> Option<String> {
        self.source_alias
            .as_ref()
            .map(|x| x.clone())
            .or_else(|| self.qualifier.table.as_ref().map(|x| x.clone()))
            .or_else(|| match &self.source {
                AvailableColumnSource::Schema(c) => c.table_name.as_ref().map(|x| x.clone()),
                AvailableColumnSource::Unresolved { .. } => None,
            })
    }

    /// Check if the column source name matches the other column source name
    pub fn same_source(&self, other: &ResolvedColumn<'_>) -> bool {
        self.source_name()
            .map_or(false, |x| other.matches_source_name(&x))
    }

    /// Check if the column qualifier matches the other qualifier
    pub fn matches_qualifier(&self, qualifier: &Option<Vec<String>>) -> bool {
        let Some(qualifier) = qualifier else {
            return true;
        };
        match qualifier.len() {
            0 => true,
            1 => Some(&qualifier[0]) == self.source_name().as_ref(),
            _ => {
                Some(&qualifier[0]) == self.source_name().as_ref()
                    && Some(&qualifier[1]) == self.schema_name()
            }
        }
    }

    /// Get the completion detail for the column
    pub fn detail(&self) -> String {
        match &self.source {
            AvailableColumnSource::Schema(c) => {
                detail(&Qualifier::from(c), &c.column_name, &Some(c.data_type))
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

pub fn get_scope_available_columns(ctx: &Context<'_>) -> Vec<AvailableColumn> {
    let mut cols = Vec::new();

    // Find all exposed columns from CTE's, FROM tables/subqueries, etc.
    let available = ctx.scope.resolve_available_columns(ctx.schema);

    // If no columns are available, add all columns from the schema.
    if available.is_empty() {
        for col in ctx.schema.get_columns().iter() {
            cols.push(AvailableColumn::from(col.clone()));
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
