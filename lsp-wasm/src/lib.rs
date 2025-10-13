use js_sys::Promise;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen as swb;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use querent_lsp::{LspRequest, LspServer};

mod catalog;
mod engine;
pub use catalog::*;

use crate::engine::{DialectName, WasmEngineProvider};

#[wasm_bindgen]
pub struct WasmLspServer {
    engine_provider: WasmEngineProvider,
    server: Rc<LspServer<WasmEngineProvider>>,
    serializer: Rc<swb::Serializer>,
}

#[wasm_bindgen]
impl WasmLspServer {
    #[wasm_bindgen(constructor)]
    pub fn new(catalog_reader: JsCatalog) -> Self {
        let engine_provider = WasmEngineProvider::new(catalog_reader);
        Self {
            engine_provider: engine_provider.clone(),
            server: Rc::new(LspServer::new(engine_provider)),
            serializer: Rc::new(swb::Serializer::json_compatible()),
        }
    }

    #[wasm_bindgen]
    pub fn set_document_dialect(&mut self, uri: String, kind: DialectName) {
        self.engine_provider.set_document_dialect(uri, kind);
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
