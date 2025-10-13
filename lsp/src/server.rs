use futures::lock::Mutex;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

use querent_core::{
    catalog::CatalogRead,
    doc::Content,
    engine::{Completion, Engine},
};

use crate::{LspRequest, LspResponse, response::LspResponseCompletions};

pub trait DocEngineProvider {
    type Catalog: CatalogRead;
    fn get(&self, uri: String) -> impl Future<Output = Option<Engine<Self::Catalog>>>;
}

#[derive(Clone)]
pub struct LspServer<E> {
    engines: E,
    documents: Arc<Mutex<HashMap<String, Content>>>,
    capabilities: serde_json::Value,
}

impl<E: DocEngineProvider> LspServer<E> {
    pub fn new(engines: E) -> Self {
        Self {
            engines,
            documents: Arc::new(Mutex::new(HashMap::new())),
            capabilities: json!({
                "completionProvider": {
                    "triggerCharacters": [";", "\n", ".", " ", ","]
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

    pub async fn handle_json_rpc(&self, req: LspRequest) -> Option<LspResponse> {
        match req {
            LspRequest::Initialize(req) => Some(LspResponse::result(
                req.id,
                json!({ "capabilities": self.capabilities }),
            )),
            LspRequest::Initialized(_) => None,
            LspRequest::DidOpen(req) => {
                let params = req.params;
                let mut docs = self.documents.lock().await;
                docs.insert(
                    params.text_document.uri.to_string(),
                    Content::new(&params.text_document.text),
                );
                None
            }
            LspRequest::DidClose(req) => {
                let params = req.params;
                let mut docs = self.documents.lock().await;
                docs.remove(&params.text_document.uri.to_string());
                None
            }
            LspRequest::DidChange(req) => {
                let params = req.params;
                let mut docs = self.documents.lock().await;
                let doc = docs.get_mut(&params.text_document.uri.to_string())?;
                for change in params.content_changes {
                    if let Some(range) = change.range {
                        doc.apply_edit(
                            (range.start.line as usize, range.start.character as usize),
                            (range.end.line as usize, range.end.character as usize),
                            &change.text,
                        );
                    }
                }
                None
            }
            LspRequest::Completion(req) => {
                let params = req.params;
                let uri = params.text_document.uri.to_string();
                let mut docs = self.documents.lock().await;
                let doc = docs.get_mut(&uri)?;
                doc.set_cursor((
                    params.position.line as usize,
                    params.position.character as usize,
                ));
                let Some(engine) = self.engines.get(uri.clone()).await else {
                    return None;
                };

                match engine.complete(&doc).await {
                    Ok(completions) => {
                        let items = completions
                            .into_iter()
                            .enumerate()
                            .map(|(i, c)| completion_from_engine(&doc, c, i))
                            .collect();
                        Some(LspResponse::result(
                            req.id,
                            LspResponseCompletions::new(items),
                        ))
                    }
                    Err(e) => return Some(LspResponse::error(req.id, e.to_string())),
                }
            }
            LspRequest::Shutdown(_) => {
                self.documents.lock().await.clear();
                None
            }
            LspRequest::CancelRequest(_) => None,
            LspRequest::SetTrace(_) => None,
        }
    }

    pub async fn debug_get_documents(&self) -> Vec<(String, String, usize)> {
        self.documents
            .lock()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.content().to_string(), v.cursor()))
            .collect()
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
