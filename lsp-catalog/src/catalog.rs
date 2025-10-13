use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use querent_core::catalog::{CatalogReadSync, schema};
use querent_lsp::{LspRequestEnvelope, LspResponse};

pub struct LspCatalog<F> {
    catalog: CatalogCache,
    pending: HashMap<u64, LspCatalogRequest>,
    request: F,
}

#[derive(Default)]
pub struct CatalogCache {
    /// schema name list
    schemas: Vec<String>,
    /// schema -> table
    tables: HashMap<String, Vec<schema::Table>>,
    /// (schema, table) -> column
    columns: HashMap<(String, String), Vec<schema::Column>>,
}

impl<F> LspCatalog<F> {
    pub fn new(request: F) -> Self {
        Self {
            catalog: CatalogCache::default(),
            pending: HashMap::new(),
            request,
        }
    }

    pub fn handle_response(&mut self, mut msg: LspResponse) -> Option<LspResponse> {
        let id = msg.id.take()?;
        let request = self.pending.remove(&id)?;
        let result = msg.result.take()?;
        match handle_catalog_request_response(&mut self.catalog, request, result) {
            Ok(_) => None,
            Err(e) => Some(LspResponse::error(Some(id), e)),
        }
    }
}

impl<F> CatalogReadSync for LspCatalog<F>
where
    F: Fn(LspCatalogRequest),
{
    fn list_schemas(&self) -> Vec<String> {
        (&self.request)(LspCatalogRequest::ListSchemas(LspRequestEnvelope::new(
            None,
            (),
        )));
        self.catalog.schemas.clone()
    }
    fn list_tables(&self, schema: &str) -> Vec<String> {
        (&self.request)(LspCatalogRequest::ListTables(LspRequestEnvelope::new(
            None,
            SchemaIdentifier {
                schema: Some(schema.to_string()),
            },
        )));
        self.catalog
            .tables
            .get(schema)
            .map(|tables| tables.iter().map(|t| t.table_name.clone()).collect())
            .unwrap_or_default()
    }
    fn list_columns(&self, table: &str, schema: &str) -> Vec<schema::Column> {
        (&self.request)(LspCatalogRequest::ListColumns(LspRequestEnvelope::new(
            None,
            TableIdentifier {
                table: table.to_string(),
                schema: Some(schema.to_string()),
            },
        )));
        self.catalog
            .columns
            .get(&(schema.to_string(), table.to_string()))
            .cloned()
            .unwrap_or_default()
    }
    fn get_table(&self, table: &str, schema: &str) -> Option<schema::Table> {
        (&self.request)(LspCatalogRequest::GetTable(LspRequestEnvelope::new(
            None,
            TableIdentifier {
                table: table.to_string(),
                schema: Some(schema.to_string()),
            },
        )));
        self.catalog
            .tables
            .get(schema)
            .and_then(|tables| tables.iter().find(|t| t.table_name == table).cloned())
    }
}

// ---------------- Messages ----------------
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum LspCatalogRequest {
    #[serde(rename = "catalog/listSchemas")]
    ListSchemas(LspRequestEnvelope<()>),
    #[serde(rename = "catalog/listTables")]
    ListTables(LspRequestEnvelope<SchemaIdentifier>),
    #[serde(rename = "catalog/listColumns")]
    ListColumns(LspRequestEnvelope<TableIdentifier>),
    #[serde(rename = "catalog/getTable")]
    GetTable(LspRequestEnvelope<TableIdentifier>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaIdentifier {
    pub schema: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableIdentifier {
    pub table: String,
    pub schema: Option<String>,
}

fn handle_catalog_request_response(
    catalog: &mut CatalogCache,
    req: LspCatalogRequest,
    result: serde_json::Value,
) -> Result<(), String> {
    match req {
        LspCatalogRequest::ListSchemas(_) => {
            let schemas =
                serde_json::from_value::<Vec<String>>(result).map_err(|e| e.to_string())?;
            catalog.schemas = schemas;
        }
        LspCatalogRequest::ListTables(req) => {
            let schema = req.params.schema.ok_or("Schema is required")?;
            let tables =
                serde_json::from_value::<Vec<schema::Table>>(result).map_err(|e| e.to_string())?;
            catalog.tables.insert(schema, tables);
        }
        LspCatalogRequest::GetTable(req) => {
            let schema = req.params.schema.ok_or("Schema is required")?;
            let table =
                serde_json::from_value::<schema::Table>(result).map_err(|e| e.to_string())?;
            if let Some(tables) = catalog.tables.get_mut(&schema) {
                if let Some(t) = tables.iter_mut().find(|t| t.table_name == table.table_name) {
                    *t = table;
                } else {
                    tables.push(table);
                }
            } else {
                catalog.tables.insert(schema, vec![table]);
            }
        }
        LspCatalogRequest::ListColumns(req) => {
            let schema = req.params.schema.ok_or("Schema is required")?;
            let table = req.params.table;
            let columns =
                serde_json::from_value::<Vec<schema::Column>>(result).map_err(|e| e.to_string())?;
            catalog.columns.insert((schema, table), columns);
        }
    };
    Ok(())
}
