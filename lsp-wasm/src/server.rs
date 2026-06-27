use std::cell::RefCell;
use std::rc::Rc;

use js_sys::Promise;
use serde::Deserialize;
use serde::Serialize;
use serde_wasm_bindgen as swb;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use crate::{provider::LspProvider, types};

#[wasm_bindgen]
pub struct LspServer {
    server: Rc<RefCell<querent_lsp::LspServer<LspProvider>>>,
    serializer: Rc<swb::Serializer>,
}

#[wasm_bindgen]
impl LspServer {
    #[wasm_bindgen(constructor)]
    pub fn new(engine_provider: &LspProvider, config: Option<types::JsLspServerConfig>) -> Self {
        Self {
            server: Rc::new(RefCell::new(querent_lsp::LspServer::new(
                engine_provider.clone(),
                config
                    .map(|c| c.try_into_rs().unwrap_or_default())
                    .unwrap_or_default(),
            ))),
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
            let response = server.borrow_mut().handle_json_rpc(request).await;
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
    Lsp(querent_lsp::LspRequest),
}
