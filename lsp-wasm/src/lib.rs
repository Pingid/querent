use js_sys::Promise;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen as swb;
use std::{rc::Rc, sync::Arc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use querent_core::{catalog::InMemoryCatalog, dialect::Ansi, engine::Engine};
use querent_lsp::{DocEngineProvider, LspRequest, LspServer};

#[wasm_bindgen]
pub struct WasmLspServer {
    server: Rc<LspServer<WasmEngineProvider>>,
    serializer: Rc<swb::Serializer>,
}

#[wasm_bindgen]
impl WasmLspServer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            server: Rc::new(LspServer::new(WasmEngineProvider {})),
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

pub struct WasmEngineProvider {}
impl DocEngineProvider for WasmEngineProvider {
    fn get(
        &self,
        _uri: String,
    ) -> std::pin::Pin<Box<dyn Future<Output = Option<Arc<Engine>>> + Send + '_>> {
        Box::pin(async move {
            Some(Arc::new(Engine::new(
                Box::new(InMemoryCatalog::default()),
                Ansi::default().spec,
            )))
        })
    }
}

// fn log_error(error: String) {
//     web_sys::console::error_1(&format!("{}", error).into());
// }

// fn log_info(info: String) {
//     web_sys::console::log_1(&format!("{}", info).into());
// }
