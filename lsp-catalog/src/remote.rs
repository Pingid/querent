use serde::{Deserialize, Serialize};
use std::sync::Arc;

use querent_core::catalog::{CatalogRead, CatalogReadResult, schema};

pub struct RemoteCatalog<F>(Arc<F>);

impl<'a, F> CatalogRead for RemoteCatalog<F>
where
    F: Fn(RemoteCatalogRequest) -> CatalogReadResult<'a, RemoteCatalogResponse> + Send + Sync,
{
    fn list_schemas(&self) -> CatalogReadResult<'_, Vec<String>> {
        let f = Arc::clone(&self.0);
        Box::pin(async move {
            match f(RemoteCatalogRequest::ListSchemas).await {
                Ok(RemoteCatalogResponse::ListSchemas { schemas }) => Ok(schemas),
                Err(e) => Err(e),
                _ => Ok(vec![]),
            }
        })
    }
    fn list_tables(&self, schema: &str) -> CatalogReadResult<'_, Vec<String>> {
        let f = Arc::clone(&self.0);
        let schema = schema.to_string();
        Box::pin(async move {
            match f(RemoteCatalogRequest::ListTables { schema: schema }).await {
                Ok(RemoteCatalogResponse::ListTables { tables }) => Ok(tables),
                Err(e) => Err(e),
                _ => Ok(vec![]),
            }
        })
    }
    fn get_table(&self, table: &str, schema: &str) -> CatalogReadResult<'_, Option<schema::Table>> {
        let f = Arc::clone(&self.0);
        let schema = schema.to_string();
        let table = table.to_string();
        Box::pin(async move {
            match f(RemoteCatalogRequest::GetTable { table, schema }).await {
                Ok(RemoteCatalogResponse::GetTable { table }) => Ok(table),
                Err(e) => Err(e),
                _ => Ok(None),
            }
        })
    }

    fn list_columns(
        &self,
        table: &str,
        schema: &str,
    ) -> CatalogReadResult<'_, Vec<schema::Column>> {
        let f = Arc::clone(&self.0);
        let schema = schema.to_string();
        let table = table.to_string();
        Box::pin(async move {
            match f(RemoteCatalogRequest::ListColumns { table, schema }).await {
                Ok(RemoteCatalogResponse::ListColumns { columns }) => Ok(columns),
                Err(e) => Err(e),
                _ => Ok(vec![]),
            }
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum RemoteCatalogRequest {
    ListSchemas,
    ListTables { schema: String },
    ListColumns { table: String, schema: String },
    GetTable { table: String, schema: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum RemoteCatalogResponse {
    ListSchemas { schemas: Vec<String> },
    ListTables { tables: Vec<String> },
    ListColumns { columns: Vec<schema::Column> },
    GetTable { table: Option<schema::Table> },
    Error { message: String },
}
