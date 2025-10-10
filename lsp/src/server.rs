use futures::lock::Mutex;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

use querent_core::{doc::Content, engine::Completion};

use crate::{
    engines::Engines,
    lsp_protocol::{LspJsonRequest, LspJsonResponse},
    proto::{ProtoRequest, ProtoResponse},
};

pub struct Document {
    pub content: Content,
}

#[derive(Clone)]
pub struct LspServer {
    engines: Engines,
    documents: Arc<Mutex<HashMap<String, Document>>>,
    capabilities: serde_json::Value,
}

impl LspServer {
    pub fn new() -> Self {
        Self {
            engines: Engines::new(),
            documents: Arc::new(Mutex::new(HashMap::new())),
            capabilities: json!({
                "completionProvider": {
                    "triggerCharacters": [".", " ", ","]
                },
                "textDocumentSync": {
                    "openClose": true,
                    "change": 2
                },
                "general": {
                    "positionEncodings": ["utf-16"]
                }
            }),
        }
    }

    pub async fn handle_proto_request(&self, req: ProtoRequest) -> Option<ProtoResponse> {
        match req {
            ProtoRequest::Lsp(req) => self.handle_json_rpc(req).await.map(ProtoResponse),
            ProtoRequest::Engine(req) => self.engines.handle(req).await.map(ProtoResponse),
            // ProtoRequest::Engine(req) => None,
        }
    }

    pub async fn handle_json_rpc(&self, req: LspJsonRequest) -> Option<LspJsonResponse> {
        match req {
            LspJsonRequest::Initialize(req) => Some(LspJsonResponse::result(
                req.id,
                json!({ "capabilities": self.capabilities }),
            )),
            LspJsonRequest::Initialized(_) => None,
            LspJsonRequest::DidOpen(req) => {
                let params = req.params?;
                let mut docs = self.documents.lock().await;
                docs.insert(
                    params.text_document.uri.to_string(),
                    Document {
                        content: Content::new(&params.text_document.text),
                    },
                );
                None
            }
            LspJsonRequest::DidClose(req) => {
                let params = req.params?;
                let mut docs = self.documents.lock().await;
                docs.remove(&params.text_document.uri.to_string());
                None
            }
            LspJsonRequest::DidChange(req) => {
                let params = req.params?;
                let mut docs = self.documents.lock().await;
                let doc = docs.get_mut(&params.text_document.uri.to_string())?;
                for change in params.content_changes {
                    if let Some(range) = change.range {
                        doc.content.apply_edit(
                            (range.start.line as usize, range.start.character as usize),
                            (range.end.line as usize, range.end.character as usize),
                            &change.text,
                        );
                    }
                }
                None
            }
            LspJsonRequest::Completion(req) => {
                let params = req.params?;
                let uri = params.text_document.uri.to_string();
                let mut docs = self.documents.lock().await;
                let doc = docs.get_mut(&uri)?;
                doc.content.set_cursor((
                    params.position.line as usize,
                    params.position.character as usize,
                ));

                let completions = self.engines.get(&uri).await.complete(&doc.content).await;

                let items = completions
                    .into_iter()
                    .enumerate()
                    .map(|(i, c)| completion_from_engine(&doc.content, c, i))
                    .collect();
                Some(LspJsonResponse::completions(req.id, items))
            }
            LspJsonRequest::Shutdown(_) => {
                self.documents.lock().await.clear();
                None
            }
            LspJsonRequest::CancelRequest(_) => None,
            LspJsonRequest::SetTrace(_) => None,
        }
    }
}

fn completion_from_engine(
    doc: &Content,
    completion: Completion,
    index: usize,
) -> lsp_types::CompletionItem {
    let start = doc.get_line_col(completion.replace.start);
    let end = doc.get_line_col(completion.replace.end);

    let kind = match &completion.kind {
        querent_core::engine::CompletionKind::Keyword => lsp_types::CompletionItemKind::KEYWORD,
        querent_core::engine::CompletionKind::Table(_) => lsp_types::CompletionItemKind::CLASS,
        querent_core::engine::CompletionKind::Column(_) => lsp_types::CompletionItemKind::FIELD,
        querent_core::engine::CompletionKind::Function => lsp_types::CompletionItemKind::FUNCTION,
        querent_core::engine::CompletionKind::Operator => lsp_types::CompletionItemKind::OPERATOR,
    };

    lsp_types::CompletionItem {
        label: completion.label,
        insert_text: Some(completion.insert_text.clone()),
        filter_text: completion.filter_text,
        kind: Some(kind),
        sort_text: Some(format!("{:05}", index)),
        text_edit: Some(lsp_types::CompletionTextEdit::Edit(lsp_types::TextEdit {
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: start.0 as u32,
                    character: start.1 as u32,
                },
                end: lsp_types::Position {
                    line: end.0 as u32,
                    character: end.1 as u32,
                },
            },
            new_text: completion.insert_text,
        })),
        ..Default::default()
    }
}
