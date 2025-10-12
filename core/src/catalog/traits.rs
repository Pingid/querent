use std::future::Future;
use std::future::ready;
use std::pin::Pin;

use crate::catalog::schema;

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type CatalogResult<'a, T> = BoxedFuture<'a, Result<T, CatalogError>>;

#[derive(Debug)]
pub enum CatalogError {
    NotFound,
    Backend(String),
    Other(String),
}

pub trait CatalogRead {
    fn list_schemas(&self) -> BoxedFuture<'_, Vec<String>>;
    fn list_tables(&self, schema: &str) -> BoxedFuture<'_, Vec<String>>;
    fn list_columns(&self, table: &str, schema: &str) -> BoxedFuture<'_, Vec<schema::Column>>;
    fn get_table(&self, table: &str, schema: &str) -> BoxedFuture<'_, Option<schema::Table>>;
    fn close(&self) -> BoxedFuture<'_, ()> {
        Box::pin(ready(()))
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
    fn list_schemas(&self) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(ready(self.list_schemas()))
    }

    fn list_tables(&self, schema: &str) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(ready(self.list_tables(schema)))
    }

    fn list_columns(
        &self,
        table: &str,
        schema: &str,
    ) -> Pin<Box<dyn Future<Output = Vec<schema::Column>> + Send + '_>> {
        Box::pin(ready(self.list_columns(table, schema)))
    }

    fn get_table(
        &self,
        table: &str,
        schema: &str,
    ) -> Pin<Box<dyn Future<Output = Option<schema::Table>> + Send + '_>> {
        Box::pin(ready(self.get_table(table, schema)))
    }
}
