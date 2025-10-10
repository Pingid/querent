use serde::{Deserialize, Serialize};

use crate::engines::EngineRequest;
use crate::lsp_protocol::{LspJsonRequest, LspJsonResponse};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProtoRequest {
    Engine(EngineRequest),
    Lsp(LspJsonRequest),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtoResponse(pub LspJsonResponse);
