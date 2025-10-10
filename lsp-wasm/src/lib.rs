use js_sys::Promise;
use querent_core::{catalog::InMemoryCatalog, dialect::Ansi, engine::Engine};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen as swb;
use std::{rc::Rc, sync::Arc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use querent_lsp::{DocEngineProvider, LspRequest, LspServer};

#[wasm_bindgen]
pub struct WasmLspServer {
    server: Rc<LspServer<WasmEngineProvider>>,
}

#[wasm_bindgen]
impl WasmLspServer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            server: Rc::new(LspServer::new(WasmEngineProvider {})),
        }
    }

    #[wasm_bindgen]
    pub fn handle(&self, msg: JsValue) -> Promise {
        let server = Rc::clone(&self.server);
        future_to_promise(async move {
            let msg_result = swb::from_value::<WasmLspRequest>(msg.clone());

            let request = match msg_result {
                Ok(WasmLspRequest::Lsp(req)) => req,
                Err(_) => match swb::from_value::<LspRequest>(msg) {
                    Ok(req) => req,
                    Err(e) => {
                        log_error(format!("Failed to deserialize LSP request: {:?}", e));
                        return Err(JsValue::from_str(&format!(
                            "Invalid request format: {:?}",
                            e
                        )));
                    }
                },
            };

            let response = server.handle_json_rpc(request).await;
            let ser = swb::Serializer::json_compatible();
            match response {
                Some(response) => response.serialize(&ser).map_err(|e| {
                    log_error(format!("Failed to serialize LSP response: {:?}", e));
                    JsValue::null()
                }),
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

pub fn log_error(error: String) {
    web_sys::console::error_1(&format!("{}", error).into());
}

pub fn log_info(info: String) {
    web_sys::console::log_1(&format!("{}", info).into());
}
