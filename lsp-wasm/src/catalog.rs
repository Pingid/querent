use js_sys::{Function, Promise, Reflect};
use querent_core::catalog::CatalogError;
use querent_core::catalog::CatalogRead;
use querent_core::catalog::CatalogReadResult;
use querent_core::catalog::CatalogResult;
use serde_wasm_bindgen as swb;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ListSchemas")]
    pub type ListSchemas;
    #[wasm_bindgen(typescript_type = "ListTables")]
    pub type ListTables;
    #[wasm_bindgen(typescript_type = "ListColumns")]
    pub type ListColumns;
    #[wasm_bindgen(typescript_type = "GetTable")]
    pub type GetTable;
    #[wasm_bindgen(typescript_type = "CatalogApi")]
    pub type CatalogApi;
}

#[wasm_bindgen(typescript_custom_section)]
const TS: &str = r#"
export type ListSchemas = (uri: string) => Promise<string[]>;
export type ListTables = (uri: string, schema: string) => Promise<string[]>;
export type ListColumns = (uri: string, table: string, schema: string) => Promise<Column[]>;
export type GetTable = (uri: string, table: string, schema: string) => Promise<Table | null>;

export interface CatalogApi {
  listSchemas: ListSchemas;
  listTables: ListTables;
  listColumns: ListColumns;
  getTable: GetTable;
}
"#;

#[wasm_bindgen]
#[derive(Clone)]
pub struct JsCatalog {
    uri: String,
    list_schemas_fn: Function,
    list_tables_fn: Function,
    list_columns_fn: Function,
    get_table_fn: Function,
}

impl JsCatalog {
    pub fn set_uri(&mut self, uri: String) {
        self.uri = uri;
    }
}

#[wasm_bindgen]
impl JsCatalog {
    #[wasm_bindgen(constructor)]
    pub fn new(catalog_api: CatalogApi) -> Result<JsCatalog, JsValue> {
        let api_obj: &JsValue = catalog_api.as_ref();

        let list_schemas_fn =
            Reflect::get(api_obj, &JsValue::from_str("listSchemas"))?.dyn_into::<Function>()?;
        let list_tables_fn =
            Reflect::get(api_obj, &JsValue::from_str("listTables"))?.dyn_into::<Function>()?;
        let list_columns_fn =
            Reflect::get(api_obj, &JsValue::from_str("listColumns"))?.dyn_into::<Function>()?;
        let get_table_fn =
            Reflect::get(api_obj, &JsValue::from_str("getTable"))?.dyn_into::<Function>()?;

        Ok(Self {
            uri: "".to_string(),
            list_schemas_fn,
            list_tables_fn,
            list_columns_fn,
            get_table_fn,
        })
    }
}

impl CatalogRead for JsCatalog {
    fn list_schemas(&self) -> CatalogReadResult<'_, Vec<String>> {
        let f = self.list_schemas_fn.clone();
        let uri = self.uri.clone();
        Box::pin(async move {
            let p_val = f.call1(&JsValue::NULL, &JsValue::from_str(&uri));
            handle_result("list_schemas", p_val).await
        })
    }

    fn list_tables(&self, schema: &str) -> CatalogReadResult<'_, Vec<String>> {
        let f = self.list_tables_fn.clone();
        let uri = self.uri.clone();
        let schema = schema.to_string();
        Box::pin(async move {
            let schema_val = JsValue::from_str(&schema);
            let p_val = f.call2(&JsValue::NULL, &JsValue::from_str(&uri), &schema_val);
            handle_result("list_tables", p_val).await
        })
    }

    fn list_columns(
        &self,
        table: &str,
        schema: &str,
    ) -> CatalogReadResult<'_, Vec<querent_core::catalog::schema::Column>> {
        let f = self.list_columns_fn.clone();
        let table = table.to_string();
        let schema = schema.to_string();
        let uri = self.uri.clone();
        Box::pin(async move {
            let table_val = JsValue::from_str(&table);
            let schema_val = JsValue::from_str(&schema);
            let p_val = f.call3(
                &JsValue::NULL,
                &JsValue::from_str(&uri),
                &table_val,
                &schema_val,
            );
            handle_result("list_columns", p_val).await
        })
    }

    fn get_table(
        &self,
        table: &str,
        schema: &str,
    ) -> CatalogReadResult<'_, Option<querent_core::catalog::schema::Table>> {
        let f = self.get_table_fn.clone();
        let table = table.to_string();
        let schema = schema.to_string();
        let uri = self.uri.clone();
        Box::pin(async move {
            let table_val = JsValue::from_str(&table);
            let schema_val = JsValue::from_str(&schema);
            let p_val = f.call3(
                &JsValue::NULL,
                &JsValue::from_str(&uri),
                &table_val,
                &schema_val,
            );
            handle_result("get_table", p_val).await
        })
    }
}

async fn handle_result<T: serde::de::DeserializeOwned>(
    name: &str,
    p_val: Result<JsValue, JsValue>,
) -> CatalogResult<T> {
    let p_val = p_val.map_err(|e| {
        CatalogError::Other(e.as_string().unwrap_or(format!("Error calling {}", name)))
    })?;
    let p: Promise = p_val.dyn_into().map_err(|e| {
        CatalogError::Other(format!(
            "failed to convert result to Promise in {} err: {:?}",
            name, e
        ))
    })?;
    let out = JsFuture::from(p).await.map_err(|e| {
        CatalogError::Other(e.as_string().unwrap_or(format!("Error calling {}", name)))
    })?;
    let out: T = swb::from_value(out).map_err(|e| {
        CatalogError::Other(format!(
            "Failed to deserialize result in {} err: {}",
            name, e
        ))
    })?;
    Ok(out)
}
