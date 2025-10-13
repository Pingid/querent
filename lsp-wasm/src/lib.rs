use js_sys::Promise;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen as swb;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use querent_core::{dialect::Ansi, engine::Engine};
use querent_lsp::{DocEngineProvider, LspRequest, LspServer};

mod catalog;
mod types;

pub use catalog::*;
pub use querent_core::catalog::schema::*;

#[wasm_bindgen]
pub struct WasmLspServer {
    server: Rc<LspServer<CatalogProvider>>,
    serializer: Rc<swb::Serializer>,
}

#[wasm_bindgen]
impl WasmLspServer {
    #[wasm_bindgen(constructor)]
    pub fn new(catalog_reader: JsCatalog) -> Self {
        let catalog_provider = CatalogProvider::new(catalog_reader);
        Self {
            server: Rc::new(LspServer::new(catalog_provider)),
            serializer: Rc::new(swb::Serializer::json_compatible()),
        }
    }

    #[wasm_bindgen]
    pub fn handle(&self, msg: JsValue) -> Promise {
        let server = Rc::clone(&self.server);
        let serializer = Rc::clone(&self.serializer);
        future_to_promise(async move {
            let msg_result = swb::from_value::<WasmLspRequest>(msg);

            let request = match msg_result {
                Ok(WasmLspRequest::Lsp(req)) => req,
                Err(e) => {
                    let err = format!("Failed to deserialize LSP request: {:?}", e);
                    return Err(JsValue::from_str(&err));
                }
            };
            let response = server.handle_json_rpc(request).await;
            match response {
                Some(response) => response
                    .serialize(&*serializer)
                    .map_err(|e| format!("Failed to serialize LSP response: {:?}", e).into()),
                None => Ok(JsValue::null()),
            }
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WasmLspRequest {
    Lsp(LspRequest),
}

#[wasm_bindgen]
pub struct CatalogProvider {
    catalog_reader: JsCatalog,
    readers: Rc<RefCell<HashMap<String, Engine<JsCatalog>>>>,
}

impl CatalogProvider {
    pub fn new(catalog_reader: JsCatalog) -> Self {
        Self {
            catalog_reader,
            readers: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}

impl DocEngineProvider for CatalogProvider {
    type Catalog = JsCatalog;
    fn get(&self, uri: String) -> Option<Engine<Self::Catalog>> {
        if let Some(reader) = self.readers.borrow().get(&uri) {
            return Some(reader.clone());
        }
        let mut catalog_reader = self.catalog_reader.clone();
        catalog_reader.set_uri(uri);
        Some(Engine::new(catalog_reader, Ansi::default().spec))
    }
}
