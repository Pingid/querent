use std::collections::HashMap;

use crate::catalog::{CatalogReadSync, schema};

pub struct InMemoryCatalog {
    schemas: HashMap<String, schema::Schema>,
}

impl InMemoryCatalog {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    pub fn add_schema(&mut self, schema: schema::Schema) {
        self.schemas.insert(schema.name.clone(), schema);
    }

    pub fn add_table(&mut self, schema: impl Into<String>, table: schema::Table) {
        let schema = schema.into();
        self.schemas
            .entry(schema.clone())
            .or_insert(schema::Schema::new(schema))
            .tables
            .push(table);
    }

    pub fn add_column(
        &mut self,
        schema: impl Into<String>,
        table: impl Into<String>,
        column: schema::Column,
    ) {
        let schema = schema.into();
        let table = table.into();
        let schem = self
            .schemas
            .entry(schema.clone())
            .or_insert(schema::Schema::new(schema.clone()));

        if let Some(t) = schem.columns.get_mut(&table) {
            if let Some(c) = t.iter_mut().find(|c| c.column_name == column.column_name) {
                *c = column;
            } else {
                t.push(column);
            }
        } else {
            schem.columns.insert(table, vec![column]);
        }
    }

    pub fn add_function(&mut self, schema: impl Into<String>, function: schema::Function) {
        let schema = schema.into();
        self.schemas
            .entry(schema.clone())
            .or_insert(schema::Schema::new(schema))
            .functions
            .push(function);
    }

    pub fn add_table_function_columns(
        &mut self,
        schema: impl Into<String>,
        function_name: impl Into<String>,
        columns: Vec<schema::Column>,
    ) {
        let schema = schema.into();
        let function_name = function_name.into();
        self.schemas
            .entry(schema.clone())
            .or_insert(schema::Schema::new(schema))
            .table_function_columns
            .entry(function_name)
            .or_insert(Vec::new())
            .extend(columns);
    }
}

impl Default for InMemoryCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl CatalogReadSync for InMemoryCatalog {
    fn list_schemas(&self) -> Vec<String> {
        let mut v: Vec<_> = self.schemas.keys().cloned().collect();
        v.sort_unstable();
        v
    }
    fn list_tables(&self, schema: &str) -> Vec<String> {
        self.schemas
            .get(schema)
            .map(|sch| sch.tables.iter().map(|t| t.table_name.clone()).collect())
            .unwrap_or_default()
    }
    fn list_columns(&self, table: &str, schema: &str) -> Vec<schema::Column> {
        if schema.is_empty() {
            return self
                .list_schemas()
                .into_iter()
                .flat_map(|s| self.list_columns(table, &s))
                .collect();
        }
        self.schemas
            .get(schema)
            .and_then(|sch| sch.columns.get(table))
            .cloned()
            .unwrap_or_default()
    }

    fn get_table(&self, table: &str, schema: &str) -> Option<schema::Table> {
        if schema.is_empty() {
            // Search all schemas for the table
            for sch in self.schemas.values() {
                if let Some(t) = sch.tables.iter().find(|t| t.table_name == table) {
                    return Some(t.clone());
                }
            }
            None
        } else {
            self.schemas
                .get(schema)
                .and_then(|sch| sch.tables.iter().find(|t| t.table_name == table))
                .cloned()
        }
    }
}

impl<'a> CatalogReadSync for &'a InMemoryCatalog {
    fn list_schemas(&self) -> Vec<String> {
        (*self).list_schemas()
    }

    fn list_tables(&self, schema: &str) -> Vec<String> {
        (*self).list_tables(schema)
    }

    fn list_columns(&self, table: &str, schema: &str) -> Vec<schema::Column> {
        (*self).list_columns(table, schema)
    }

    fn get_table(&self, table: &str, schema: &str) -> Option<schema::Table> {
        (*self).get_table(table, schema)
    }
}
