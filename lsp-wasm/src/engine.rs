use std::{cell::RefCell, collections::HashMap, rc::Rc};

use querent_core::{
    dialect::{DialectKind, DialectSpecProvider},
    engine::Engine,
};
use querent_lsp::DocEngineProvider;
use wasm_bindgen::prelude::*;

use crate::{JsCatalog, JsCatalogReader};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DialectName")]
    pub type DialectName;

}

#[wasm_bindgen(typescript_custom_section)]
const TS: &str = r#"
    export type DialectName = "ansi" | "postgres";
"#;

#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmEngineProvider {
    documents: Rc<RefCell<HashMap<String, DialectKind>>>,
    catalog_api: JsCatalog,
}

#[wasm_bindgen]
impl WasmEngineProvider {
    #[wasm_bindgen(constructor)]
    pub fn new(catalog_api: JsCatalog) -> Self {
        Self {
            documents: Rc::new(RefCell::new(HashMap::new())),
            catalog_api: catalog_api,
        }
    }

    pub fn set_document_dialect(&self, uri: String, kind: DialectName) {
        let name_string = kind.as_string().unwrap_or_default();
        let kind = DialectKind::from(name_string);
        if let Some(d) = self.documents.borrow_mut().get_mut(&uri) {
            if d.name() != kind.name() {
                *d = kind;
            }
        } else {
            self.documents.borrow_mut().insert(uri, kind);
        }
    }
}

impl DocEngineProvider for WasmEngineProvider {
    type Catalog = JsCatalogReader;
    async fn get(&self, uri: String) -> Option<Engine<Self::Catalog>> {
        if let Some(kind) = self.documents.borrow().get(&uri) {
            return Some(Engine::new(
                JsCatalogReader::new(self.catalog_api.clone(), uri),
                kind.get_spec(),
            ));
        }
        Some(Engine::new(
            JsCatalogReader::new(self.catalog_api.clone(), uri),
            DialectKind::default().get_spec(),
        ))
    }
}
