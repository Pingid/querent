use std::future::Future;
use std::future::ready;
use std::pin::Pin;

use crate::catalog::schema;

type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait CatalogRead {
    fn list_schemas(&self) -> BoxedFuture<'_, Vec<String>>;
    fn list_tables(&self, schema: Option<&str>) -> BoxedFuture<'_, Vec<String>>;
    fn list_columns(
        &self,
        table: &str,
        schema: Option<&str>,
    ) -> BoxedFuture<'_, Vec<schema::Column>>;
    fn get_table(
        &self,
        table: &str,
        schema: Option<&str>,
    ) -> BoxedFuture<'_, Option<schema::Table>>;
    fn list_functions(&self, _schema: Option<&str>) -> BoxedFuture<'_, Vec<schema::Function>> {
        Box::pin(ready(vec![]))
    }
    fn describe_table_function(
        &self,
        _name: &str,
        _schema: Option<&str>,
    ) -> BoxedFuture<'_, Vec<schema::Column>> {
        Box::pin(ready(vec![]))
    }
}

pub trait CatalogReadSync {
    fn list_schemas(&self) -> Vec<String>;
    fn list_tables(&self, schema: Option<&str>) -> Vec<String>;
    fn list_columns(&self, table: &str, schema: Option<&str>) -> Vec<schema::Column>;
    fn get_table(&self, table: &str, schema: Option<&str>) -> Option<schema::Table>;
    fn list_functions(&self, _schema: Option<&str>) -> Vec<schema::Function> {
        vec![]
    }
    fn describe_table_function(&self, _name: &str, _schema: Option<&str>) -> Vec<schema::Column> {
        vec![]
    }
}

impl<T> CatalogRead for T
where
    T: CatalogReadSync + Send + Sync,
{
    fn list_schemas(&self) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(ready(self.list_schemas()))
    }

    fn list_tables(
        &self,
        schema: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(ready(self.list_tables(schema)))
    }

    fn list_columns(
        &self,
        table: &str,
        schema: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Vec<schema::Column>> + Send + '_>> {
        Box::pin(ready(self.list_columns(table, schema)))
    }

    fn get_table(
        &self,
        table: &str,
        schema: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Option<schema::Table>> + Send + '_>> {
        Box::pin(ready(self.get_table(table, schema)))
    }

    fn list_functions(
        &self,
        schema: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Vec<schema::Function>> + Send + '_>> {
        Box::pin(ready(self.list_functions(schema)))
    }

    fn describe_table_function(
        &self,
        name: &str,
        schema: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Vec<schema::Column>> + Send + '_>> {
        Box::pin(ready(self.describe_table_function(name, schema)))
    }
}
