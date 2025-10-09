use futures::lock::Mutex;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

use querent_core::{
    catalog::InMemoryCatalog,
    dialect::Ansi,
    doc::Content,
    engine::{Completion, Engine},
};

use crate::rpc;

pub struct Document {
    pub content: Content,
}

#[derive(Clone)]
pub struct LspServer {
    documents: Arc<Mutex<HashMap<String, Document>>>,
    capabilities: serde_json::Value,
    engine: Arc<Engine<InMemoryCatalog, Ansi>>,
}

impl LspServer {
    pub fn new() -> Self {
        Self {
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
            engine: Arc::new(Engine::new(InMemoryCatalog::new(), Ansi::default())),
        }
    }

    pub async fn handle_rpc(&self, req: rpc::JsonRequest) -> Option<rpc::ResponseEnvelope> {
        match req {
            rpc::JsonRequest::Initialize(req) => Some(rpc::ResponseEnvelope::result(
                req.id,
                json!({ "capabilities": self.capabilities }),
            )),
            rpc::JsonRequest::Initialized(_) => None,
            rpc::JsonRequest::DidOpen(req) => {
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
            rpc::JsonRequest::DidClose(req) => {
                let params = req.params?;
                let mut docs = self.documents.lock().await;
                docs.remove(&params.text_document.uri.to_string());
                None
            }
            rpc::JsonRequest::DidChange(req) => {
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
            rpc::JsonRequest::Completion(req) => {
                let params = req.params?;
                let mut docs = self.documents.lock().await;
                let doc = docs.get_mut(&params.text_document.uri.to_string())?;
                doc.content.set_cursor((
                    params.position.line as usize,
                    params.position.character as usize,
                ));
                let completions = self.engine.complete(&doc.content).await;
                let items = completions
                    .into_iter()
                    .map(|c| Self::completion_from_engine(&doc.content, c))
                    .collect();
                Some(rpc::ResponseEnvelope::result(
                    req.id,
                    serde_json::to_value(rpc::CompletionResponse::new(items)).unwrap(),
                ))
            }
            rpc::JsonRequest::Shutdown(_) => {
                self.documents.lock().await.clear();
                None
            }
            rpc::JsonRequest::CancelRequest(ev) => {
                println!("CancelRequest: {:#?}", ev);
                None
            }
            rpc::JsonRequest::SetTrace(_) => None,
        }
    }
}

impl LspServer {
    fn completion_from_engine(doc: &Content, completion: Completion) -> lsp_types::CompletionItem {
        let start = doc.get_line_col(completion.replace.start);
        let end = doc.get_line_col(completion.replace.end);

        lsp_types::CompletionItem {
            label: completion.label,
            insert_text: Some(completion.insert_text.clone()),
            filter_text: completion.filter_text,
            kind: Some(lsp_types::CompletionItemKind::TEXT),
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
}
