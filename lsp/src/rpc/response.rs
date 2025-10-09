use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    jsonrpc: String,
    id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<serde_json::Value>,
}

impl ResponseEnvelope {
    pub fn result(id: Option<u64>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<u64>, error: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub items: Vec<lsp_types::CompletionItem>,
}

impl CompletionResponse {
    pub fn new(items: Vec<lsp_types::CompletionItem>) -> Self {
        Self { items }
    }
}
