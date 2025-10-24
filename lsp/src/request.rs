use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum LspRequest {
    // Lifecycle
    #[serde(rename = "initialize")]
    Initialize(LspRequestEnvelope<InitializeParams>),
    #[serde(rename = "initialized")]
    Initialized(LspRequestEnvelope<serde_json::Value>),
    #[serde(rename = "shutdown")]
    Shutdown(LspRequestEnvelope<serde_json::Value>),

    // Text Document
    #[serde(rename = "textDocument/didOpen")]
    DidOpen(LspRequestEnvelope<TextDocumentDidOpenParams>),
    #[serde(rename = "textDocument/didClose")]
    DidClose(LspRequestEnvelope<TextDocumentDidCloseParams>),
    #[serde(rename = "textDocument/didChange")]
    DidChange(LspRequestEnvelope<TextDocumentDidChangeParams>),

    // Completion
    #[serde(rename = "textDocument/completion")]
    Completion(LspRequestEnvelope<CompletionParams>),

    // Other
    #[serde(rename = "$/setTrace")]
    SetTrace(LspRequestEnvelope<serde_json::Value>),

    #[serde(rename = "$/cancelRequest")]
    CancelRequest(LspRequestEnvelope<CancelRequestParams>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspRequestEnvelope<T> {
    pub jsonrpc: String,
    pub params: T,
    pub id: Option<u64>,
}

impl<T> LspRequestEnvelope<T> {
    pub fn new(id: Option<u64>, params: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            params,
            id,
        }
    }
}

// ---------------- INITIALIZE ----------------
#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "processId")]
    pub process_id: Option<i32>,
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
    pub context: Option<lsp_types::CompletionContext>,
}

// ---------------- CANCEL REQUEST ----------------
#[derive(Debug, Serialize, Deserialize)]
pub struct CancelRequestParams {
    pub id: u64,
}
