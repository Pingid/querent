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
    fn get_schema(&self, schema: &str) -> BoxedFuture<'_, Option<&schema::Schema>>;
}

/// Iterator-based utilities to avoid materializing large Vecs
pub trait CatalogExt: CatalogRead + Send + Sync {
    fn columns_of(
        &self,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> BoxedFuture<'_, Vec<schema::Column>> {
        let schema = schema.into();
        let table = table.into();
        Box::pin(async move { self.list_columns(&table, Some(&schema)).await })
    }

    fn all_tables_iter(&self) -> BoxedFuture<'_, std::vec::IntoIter<(String, String)>> {
        Box::pin(async move {
            let schemas = self.list_schemas().await;
            let mut pairs = Vec::new();

            for sch in schemas {
                let tables = self.list_tables(Some(&sch)).await;
                pairs.extend(tables.into_iter().map(|tbl| (sch.clone(), tbl)));
            }

            pairs.into_iter()
        })
    }

    fn columns_of_iter(
        &self,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> BoxedFuture<'_, std::vec::IntoIter<schema::Column>> {
        let schema = schema.into();
        let table = table.into();
        Box::pin(async move {
            let cols = self.list_columns(&table, Some(&schema)).await;
            cols.into_iter()
        })
    }

    fn all_columns_iter(
        &self,
    ) -> BoxedFuture<'_, std::vec::IntoIter<(String, String, schema::Column)>> {
        Box::pin(async move {
            let mut rows = Vec::new();

            let schemas = self.list_schemas().await;
            for sch in schemas {
                let tables = self.list_tables(Some(&sch)).await;
                for tbl in tables {
                    let cols = self.list_columns(&tbl, Some(&sch)).await;
                    rows.extend(cols.into_iter().map({
                        let sch = sch.clone();
                        let tbl = tbl.clone();
                        move |col| (sch.clone(), tbl.clone(), col)
                    }));
                }
            }

            rows.into_iter()
        })
    }
}
impl<T: CatalogRead + Send + Sync + ?Sized> CatalogExt for T {}

pub trait CatalogReadSync {
    fn list_schemas(&self) -> Vec<String>;
    fn list_tables(&self, schema: Option<&str>) -> Vec<String>;
    fn list_columns(&self, table: &str, schema: Option<&str>) -> Vec<schema::Column>;
    fn get_schema(&self, schema: &str) -> Option<&schema::Schema>;
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

    fn get_schema(
        &self,
        schema: &str,
    ) -> Pin<Box<dyn Future<Output = Option<&schema::Schema>> + Send + '_>> {
        Box::pin(ready(self.get_schema(schema)))
    }
}
