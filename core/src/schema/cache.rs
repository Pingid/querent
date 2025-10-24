use crate::schema;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Debug, Default, Clone)]
pub struct Cache {
    pub columns: Vec<schema::Column>,
    pub tables: Vec<schema::Table>,
    pub functions: Vec<schema::Function>,
}

/// Schema cache builder helpers
impl Cache {
    pub fn add_column(&mut self, column: schema::Column) {
        self.columns.push(column);
    }

    pub fn add_table(&mut self, table: schema::Table) {
        self.tables.push(table);
    }

    pub fn add_function(&mut self, function: schema::Function) {
        self.functions.push(function);
    }
}

/// Schema cache query helpers
impl Cache {
    pub fn get_columns(&self) -> &[schema::Column] {
        &self.columns
    }

    pub fn get_tables(&self) -> &[schema::Table] {
        &self.tables
    }

    pub fn get_functions(&self) -> &[schema::Function] {
        &self.functions
    }
}
