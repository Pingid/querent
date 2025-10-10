use serde::{Deserialize, Serialize};
use std::sync::Arc;

use querent_core::catalog::{BoxedFuture, CatalogRead, schema};

pub struct RemoteCatalog<F>(Arc<F>);

impl<'a, F> CatalogRead for RemoteCatalog<F>
where
    F: Fn(RemoteCatalogRequest) -> BoxedFuture<'a, RemoteCatalogResponse> + Send + Sync,
{
    fn list_schemas(&self) -> BoxedFuture<'_, Vec<String>> {
        let f = Arc::clone(&self.0);
        Box::pin(async move {
            match f(RemoteCatalogRequest::ListSchemas).await {
                RemoteCatalogResponse::ListSchemas { schemas } => schemas,
                _ => vec![],
            }
        })
    }
    fn list_tables(&self, schema: &str) -> BoxedFuture<'_, Vec<String>> {
        let f = Arc::clone(&self.0);
        let schema = schema.to_string();
        Box::pin(async move {
            match f(RemoteCatalogRequest::ListTables { schema: schema }).await {
                RemoteCatalogResponse::ListTables { tables } => tables,
                _ => vec![],
            }
        })
    }
    fn get_table(&self, table: &str, schema: &str) -> BoxedFuture<'_, Option<schema::Table>> {
        let f = Arc::clone(&self.0);
        let schema = schema.to_string();
        let table = table.to_string();
        Box::pin(async move {
            match f(RemoteCatalogRequest::GetTable { table, schema }).await {
                RemoteCatalogResponse::GetTable { table } => table,
                _ => None,
            }
        })
    }

    fn list_columns(&self, table: &str, schema: &str) -> BoxedFuture<'_, Vec<schema::Column>> {
        let f = Arc::clone(&self.0);
        let schema = schema.to_string();
        let table = table.to_string();
        Box::pin(async move {
            match f(RemoteCatalogRequest::ListColumns { table, schema }).await {
                RemoteCatalogResponse::ListColumns { columns } => columns,
                _ => vec![],
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
}
