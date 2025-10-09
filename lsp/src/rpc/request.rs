use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum JsonRequest {
    // Lifecycle
    #[serde(rename = "initialize")]
    Initialize(RequestEnvelope<InitializeParams>),
    #[serde(rename = "initialized")]
    Initialized(RequestEnvelope<serde_json::Value>),
    #[serde(rename = "shutdown")]
    Shutdown(RequestEnvelope<serde_json::Value>),

    // Text Document
    #[serde(rename = "textDocument/didOpen")]
    DidOpen(RequestEnvelope<TextDocumentDidOpenParams>),
    #[serde(rename = "textDocument/didClose")]
    DidClose(RequestEnvelope<TextDocumentDidCloseParams>),
    #[serde(rename = "textDocument/didChange")]
    DidChange(RequestEnvelope<TextDocumentDidChangeParams>),

    // Completion
    #[serde(rename = "textDocument/completion")]
    Completion(RequestEnvelope<CompletionParams>),

    // Other
    #[serde(rename = "$/setTrace")]
    SetTrace(RequestEnvelope<serde_json::Value>),

    #[serde(rename = "$/cancelRequest")]
    CancelRequest(RequestEnvelope<CancelRequestParams>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestEnvelope<T> {
    pub jsonrpc: String,
    pub params: Option<T>,
    pub id: Option<u64>,
}

// ---------------- INITIALIZE ----------------
#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "processId")]
    pub process_id: i32,
    #[serde(rename = "rootUri")]
    pub root_uri: Option<String>,
    pub capabilities: serde_json::Value,
}

// ---------------- TEXT DOCUMENT DID OPEN ----------------
#[derive(Debug, Serialize, Deserialize)]
pub struct TextDocumentDidOpenParams {
    #[serde(rename = "textDocument")]
    pub text_document: lsp_types::TextDocumentItem,
}

// ---------------- TEXT DOCUMENT DID CLOSE ----------------
#[derive(Debug, Serialize, Deserialize)]
pub struct TextDocumentDidCloseParams {
    #[serde(rename = "textDocument")]
    pub text_document: lsp_types::TextDocumentIdentifier,
}

// ---------------- TEXT DOCUMENT DID CHANGE ----------------
#[derive(Debug, Serialize, Deserialize)]
pub struct TextDocumentDidChangeParams {
    #[serde(rename = "textDocument")]
    pub text_document: lsp_types::VersionedTextDocumentIdentifier,
    #[serde(rename = "contentChanges")]
    pub content_changes: Vec<lsp_types::TextDocumentContentChangeEvent>,
}

// ---------------- COMPLETION ----------------
#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionParams {
    #[serde(rename = "textDocument")]
    pub text_document: lsp_types::TextDocumentIdentifier,
    pub position: lsp_types::Position,
    pub context: lsp_types::CompletionContext,
}

// ---------------- CANCEL REQUEST ----------------
#[derive(Debug, Serialize, Deserialize)]
pub struct CancelRequestParams {
    pub id: u64,
}
