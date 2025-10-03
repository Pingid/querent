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
        if let Some(t) = schem.tables.iter_mut().find(|t| t.name == table) {
            t.columns.push(column);
        } else {
            schem.tables.push(schema::Table {
                name: table,
                kind: schema::TableKind::Table,
                columns: vec![column],
                foreign_keys: vec![],
                description: None,
            });
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
    fn list_tables(&self, schema: Option<&str>) -> Vec<String> {
        match schema {
            Some(s) => self
                .schemas
                .get(s)
                .map(|sch| sch.tables.iter().map(|t| t.name.clone()).collect())
                .unwrap_or_default(),
            None => self
                .schemas
                .values()
                .flat_map(|sch| sch.tables.iter().map(|t| t.name.clone()))
                .collect(),
        }
    }
    fn list_columns(&self, table: &str, schema: Option<&str>) -> Vec<schema::Column> {
        if let Some(s) = schema {
            return self
                .schemas
                .get(s)
                .and_then(|sch| sch.tables.iter().find(|t| t.name == table))
                .map(|t| t.columns.clone())
                .unwrap_or_default();
        }
        for sch in self.schemas.values() {
            if let Some(t) = sch.tables.iter().find(|t| t.name == table) {
                return t.columns.clone();
            }
        }
        Vec::new()
    }

    fn get_table(&self, table: &str, schema: Option<&str>) -> Option<schema::Table> {
        if let Some(s) = schema {
            return self
                .schemas
                .get(s)
                .and_then(|sch| sch.tables.iter().find(|t| t.name == table))
                .cloned();
        }
        for sch in self.schemas.values() {
            if let Some(t) = sch.tables.iter().find(|t| t.name == table) {
                return Some(t.clone());
            }
        }
        None
    }

    fn list_functions(&self, schema: Option<&str>) -> Vec<schema::Function> {
        if let Some(s) = schema {
            return self
                .schemas
                .get(s)
                .map(|sch| sch.functions.clone())
                .unwrap_or_default();
        }
        for sch in self.schemas.values() {
            return sch.functions.clone();
        }
        Vec::new()
    }

    fn describe_table_function(&self, name: &str, schema: Option<&str>) -> Vec<schema::Column> {
        if let Some(s) = schema {
            return self
                .schemas
                .get(s)
                .map(|sch| {
                    sch.table_function_columns
                        .get(name)
                        .cloned()
                        .unwrap_or_default()
                })
                .unwrap_or_default();
        }
        self.schemas
            .values()
            .find_map(|sch| sch.table_function_columns.get(name).cloned())
            .unwrap_or_default()
    }
}
