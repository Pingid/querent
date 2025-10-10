use serde::{Deserialize, Serialize};

use querent_lsp::LspRequestEnvelope;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum EngineRequest {
    #[serde(rename = "engine/set")]
    Set(LspRequestEnvelope<SetEngine>),
    #[serde(rename = "engine/remove")]
    Remove(LspRequestEnvelope<RemoveEngine>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetEngine {
    pub document_uri: String,
    pub uri: String,
    pub kind: EngineKind,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveEngine {
    pub document_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EngineKind {
    #[serde(rename = "postgres")]
    Postgres(PostgresCon),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostgresCon {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum EngineResponsePayload {
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "success")]
    Success,
}
