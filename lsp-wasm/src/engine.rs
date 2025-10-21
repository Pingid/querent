use serde_wasm_bindgen as swb;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::prelude::*;

use querent_core::{
    complete::Engine,
    dialect::{DialectKind, DialectSpecProvider},
    schema,
};
use querent_lsp::CompletionProvider;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DialectName")]
    pub type DialectName;
    #[wasm_bindgen(typescript_type = "Cache")]
    pub type SchemaCache;

}

#[wasm_bindgen(typescript_custom_section)]
const TS: &str = r#"
    export type DialectName = "ansi" | "postgres";
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

    pub fn set_dialect(&self, uri: String, dialect: DialectName) {
        let name_string = dialect.as_string().unwrap_or_default();
        let kind = DialectKind::from(name_string);
        let spec = kind.get_spec();
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

    pub fn set_schema(&self, uri: String, schema: SchemaCache) -> Result<(), JsValue> {
        let schema = swb::from_value::<schema::Cache>(schema.obj)
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize schema: {:?}", e)))?;
        if let Some(engine) = self.engines.borrow_mut().get_mut(&uri) {
            engine.schema = schema;
        } else {
            self.engines
                .borrow_mut()
                .insert(uri, Engine::new(DialectKind::default().get_spec(), schema));
        }
        Ok(())
    }

    pub fn remove(&self, uri: String) {
        self.engines.borrow_mut().remove(&uri);
    }
}

impl CompletionProvider for EngineProvider {
    fn complete(
        &self,
        uri: String,
        doc: &querent_core::doc::Content,
    ) -> Vec<querent_core::complete::Completion> {
        if let Some(engine) = self.engines.borrow().get(&uri) {
            engine.complete(doc).items
        } else {
            vec![]
        }
    }
}
