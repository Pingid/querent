use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use querent_core::complete::Engine;
use querent_core::complete::completion::Completion;
use querent_core::dialect::DialectKind;
use querent_core::schema;
use querent_lsp::CompletionProvider;
use serde_wasm_bindgen as swb;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DialectKind")]
    pub type DialectKindJsValue;
    #[wasm_bindgen(typescript_type = "Cache")]
    pub type CacheJsValue;

    #[wasm_bindgen(typescript_type = "Queries")]
    pub type QueriesJsValue;
}

#[wasm_bindgen(typescript_custom_section)]
const TS: &str = r#"
    export type DialectKind = "ansi" | "postgres" | "sqlite";
"#;

#[wasm_bindgen]
#[derive(Clone, Default)]
pub struct EngineProvider {
    engines: Rc<RefCell<HashMap<String, Engine>>>,
}

#[wasm_bindgen]
impl EngineProvider {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_queries(&self, dialect: DialectKindJsValue) -> Result<QueriesJsValue, JsValue> {
        let name_string = dialect.as_string().unwrap_or_default();
        let kind: DialectKind = DialectKind::from(name_string);
        let queries: Queries = kind.into();
        let js_value = serde_wasm_bindgen::to_value(&queries)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize queries: {:?}", e)))?;
        Ok(js_value.into())
    }

    pub fn set_dialect(&self, uri: String, dialect: DialectKindJsValue) {
        let name_string = dialect.as_string().unwrap_or_default();
        let kind = DialectKind::from(name_string);
        let spec = kind.spec();
        if let Some(engine) = self.engines.borrow_mut().get_mut(&uri) {
            if engine.spec.name != spec.name {
                engine.spec = spec;
            }
        } else {
            self.engines
                .borrow_mut()
                .insert(uri, Engine::new(spec, schema::Cache::default()));
        }
    }

    pub fn set_schema(&self, uri: String, schema: CacheJsValue) -> Result<(), JsValue> {
        let schema = swb::from_value::<schema::Cache>(schema.obj)
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize schema: {:?}", e)))?;
        if let Some(engine) = self.engines.borrow_mut().get_mut(&uri) {
            engine.schema = schema;
        } else {
            self.engines
                .borrow_mut()
                .insert(uri, Engine::new(DialectKind::default().spec(), schema));
        }
        Ok(())
    }

    pub fn remove(&self, uri: String) {
        self.engines.borrow_mut().remove(&uri);
    }
}

impl CompletionProvider for EngineProvider {
    fn complete(&self, uri: String, doc: &querent_core::doc::Content) -> Vec<Completion> {
        if let Some(engine) = self.engines.borrow().get(&uri) {
            engine.complete(doc).items
        } else {
            vec![]
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Queries {
    pub functions: Option<String>,
    pub tables: Option<String>,
    pub columns: Option<String>,
}

impl From<DialectKind> for Queries {
    fn from(kind: DialectKind) -> Self {
        Self {
            functions: kind.functions_query().map(|s| s.to_string()),
            tables: kind.tables_query().map(|s| s.to_string()),
            columns: kind.columns_query().map(|s| s.to_string()),
        }
    }
}
