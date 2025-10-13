use std::future::Future;
use std::future::ready;
use std::pin::Pin;

use crate::catalog::schema;

// #[cfg(not(target_arch = "wasm32"))]
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

// #[cfg(target_arch = "wasm32")]
// pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

pub type CatalogResult<T> = Result<T, CatalogError>;
pub type CatalogReadResult<'a, T> = BoxedFuture<'a, CatalogResult<T>>;

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("not found")]
    NotFound,
    #[error("backend error: {0}")]
    Backend(String),
    #[error("other error: {0}")]
    Other(String),
}

pub trait CatalogRead {
    fn list_schemas(&self) -> CatalogReadResult<'_, Vec<String>>;
    fn list_tables(&self, schema: &str) -> CatalogReadResult<'_, Vec<String>>;
    fn list_columns(&self, table: &str, schema: &str)
    -> CatalogReadResult<'_, Vec<schema::Column>>;
    fn get_table(&self, table: &str, schema: &str) -> CatalogReadResult<'_, Option<schema::Table>>;
}

pub trait CatalogConnect {
    fn connect(&self) -> CatalogReadResult<'_, ()> {
        Box::pin(ready(Ok(())))
    }
    fn close(&self) -> CatalogReadResult<'_, ()> {
        Box::pin(ready(Ok(())))
    }
}

pub trait CatalogReadSync {
    fn list_schemas(&self) -> Vec<String>;
    fn list_tables(&self, schema: &str) -> Vec<String>;
    fn list_columns(&self, table: &str, schema: &str) -> Vec<schema::Column>;
    fn get_table(&self, table: &str, schema: &str) -> Option<schema::Table>;
}

impl<T> CatalogRead for T
where
    T: CatalogReadSync + Send + Sync,
{
    fn list_schemas(&self) -> CatalogReadResult<'_, Vec<String>> {
        Box::pin(ready(Ok(self.list_schemas())))
    }

    fn list_tables(&self, schema: &str) -> CatalogReadResult<'_, Vec<String>> {
        Box::pin(ready(Ok(self.list_tables(schema))))
    }

    fn list_columns(
        &self,
        table: &str,
        schema: &str,
    ) -> CatalogReadResult<'_, Vec<schema::Column>> {
        Box::pin(ready(Ok(self.list_columns(table, schema))))
    }

    fn get_table(&self, table: &str, schema: &str) -> CatalogReadResult<'_, Option<schema::Table>> {
        Box::pin(ready(Ok(self.get_table(table, schema))))
    }
}
