use crate::dialect::SpecFunction;
use crate::schema;

#[derive(Debug, Default, Clone, Copy)]
pub struct TableName<'a> {
    pub database_name: Option<&'a str>,
    pub schema_name: Option<&'a str>,
    pub table_name: Option<&'a str>,
}
impl<'a> TableName<'a> {
    pub fn matches_table(&self, table: &'a schema::Table) -> bool {
        if self.table_name != Some(table.table_name.as_str()) {
            return false;
        }
        if self.schema_name.is_some()
            && self.schema_name != table.schema_name.as_ref().map(|s| s.as_str())
        {
            return false;
        }
        true
    }
}

impl<'a> From<Vec<&'a str>> for TableName<'a> {
    fn from(parts: Vec<&'a str>) -> Self {
        Self {
            database_name: get_nth_last(&parts, 3),
            schema_name: get_nth_last(&parts, 2),
            table_name: get_nth_last(&parts, 1),
        }
    }
}

impl<'a> From<&'a schema::Table> for TableName<'a> {
    fn from(table: &'a schema::Table) -> Self {
        Self {
            database_name: None,
            schema_name: table.schema_name.as_ref().map(|s| s.as_str()),
            table_name: Some(table.table_name.as_str()),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ColumnName<'a> {
    pub database_name: Option<&'a str>,
    pub schema_name: Option<&'a str>,
    pub table_name: Option<&'a str>,
    pub column_name: Option<&'a str>,
}
impl<'a> ColumnName<'a> {
    pub fn is_star(&self) -> bool {
        self.column_name.map_or(false, |c| c == "*")
    }

    /// Returns true if this column reference has no table qualifier
    pub fn is_unqualified(&self) -> bool {
        self.table_name.is_none()
    }

    /// Returns true if the table qualifier matches the given table name
    pub fn is_from_table(&self, table: impl Into<TableName<'a>>) -> bool {
        let table = table.into();
        self.table_name == table.table_name
            && eq_if_some(self.database_name, table.database_name)
            && eq_if_some(self.schema_name, table.schema_name)
    }

    /// Returns true if the table qualifier matches either the table name or the given alias
    pub fn matches_table_or_alias(
        &self, table: impl Into<TableName<'a>>, alias: Option<&'a str>,
    ) -> bool {
        // Unqualified columns match any table
        if self.is_unqualified() {
            return true;
        }
        // Check if it matches the alias first
        if let Some(alias) = alias {
            if self.table_name == Some(alias) {
                return true;
            }
        }
        // Otherwise check if it matches the table name
        self.is_from_table(table)
    }

    /// Returns true if this column reference can refer to the provided alias
    pub fn matches_alias(&self, alias: Option<&'a str>) -> bool {
        self.is_unqualified() || alias.is_some_and(|alias| self.table_name == Some(alias))
    }

    /// Returns true if the column name matches (considering star expansion)
    pub fn matches_column(&self, column: &ColumnName<'a>) -> bool {
        self.is_star() || self.column_name == column.column_name
    }
}

impl<'a> From<Vec<&'a str>> for ColumnName<'a> {
    fn from(name: Vec<&'a str>) -> Self {
        Self {
            database_name: get_nth_last(&name, 4),
            schema_name: get_nth_last(&name, 3),
            table_name: get_nth_last(&name, 2),
            column_name: get_nth_last(&name, 1),
        }
    }
}

impl<'a> From<&'a str> for ColumnName<'a> {
    fn from(name: &'a str) -> Self {
        Self {
            column_name: Some(name),
            ..Default::default()
        }
    }
}

impl<'a> From<&'a schema::Column> for ColumnName<'a> {
    fn from(column: &'a schema::Column) -> Self {
        Self {
            database_name: None,
            schema_name: column.schema_name.as_ref().map(|s| s.as_str()),
            table_name: column.table_name.as_ref().map(|s| s.as_str()),
            column_name: Some(column.column_name.as_str()),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FunctionName<'a> {
    pub database_name: Option<&'a str>,
    pub schema_name: Option<&'a str>,
    pub function_name: Option<&'a str>,
}

impl<'a> From<Vec<&'a str>> for FunctionName<'a> {
    fn from(name: Vec<&'a str>) -> Self {
        Self {
            database_name: get_nth_last(&name, 3),
            schema_name: get_nth_last(&name, 2),
            function_name: get_nth_last(&name, 1),
        }
    }
}

impl<'a> From<&'a str> for FunctionName<'a> {
    fn from(name: &'a str) -> Self {
        Self {
            database_name: None,
            schema_name: None,
            function_name: Some(name),
        }
    }
}

impl<'a> From<&'a schema::Function> for FunctionName<'a> {
    fn from(function: &'a schema::Function) -> Self {
        Self {
            database_name: function.database_name.as_ref().map(|s| s.as_str()),
            schema_name: function.schema_name.as_ref().map(|s| s.as_str()),
            function_name: Some(function.function_name.as_str()),
        }
    }
}

fn eq_if_some<T: PartialEq>(a: Option<T>, b: Option<T>) -> bool {
    !matches!((a, b), (Some(a), Some(b)) if a != b)
}

fn get_nth_last<T: Copy>(parts: &[T], n: usize) -> Option<T> {
    if parts.len() < n {
        return None;
    }
    Some(parts[parts.len() - n])
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResolvedFunction<'schema> {
    Spec(&'schema SpecFunction),
    Schema(&'schema schema::Function),
}

impl<'schema> ResolvedFunction<'schema> {
    pub fn return_type(&self) -> &'schema schema::FunctionReturnType {
        match self {
            ResolvedFunction::Spec(func) => &func.return_type,
            ResolvedFunction::Schema(func) => &func.return_type,
        }
    }
    pub fn function_name(&self) -> String {
        match self {
            ResolvedFunction::Spec(func) => func.function_name.to_string(),
            ResolvedFunction::Schema(func) => func.function_name.to_string(),
        }
    }
    pub fn parameter_types(&self) -> Vec<schema::DataType> {
        match self {
            ResolvedFunction::Spec(func) => func.parameter_types.to_vec(),
            ResolvedFunction::Schema(func) => func.parameter_types.to_vec(),
        }
    }
    fn description(&self) -> Option<String> {
        match self {
            ResolvedFunction::Spec(func) => Some(func.description.to_string()),
            ResolvedFunction::Schema(func) => func.description.clone(),
        }
    }
}
