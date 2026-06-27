use std::collections::HashMap;
use std::sync::Arc;

use futures::lock::Mutex;
use querent_core::complete::types::Completion;
use querent_core::complete::types::CompletionKind;
use querent_core::complete::types::InsertTextFormat;
use querent_core::doc::Content;
use serde_json::json;

use crate::LspRequest;
use crate::LspResponse;
use crate::response::LspResponseCompletions;

pub trait CompletionProvider {
    fn complete(&mut self, uri: String, doc: &Content) -> Vec<Completion>;
}

#[derive(Clone)]
pub struct LspServer<E> {
    engines: E,
    documents: Arc<Mutex<HashMap<String, Content>>>,
    capabilities: serde_json::Value,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct LspServerConfig {
    #[cfg_attr(feature = "typescript", ts(type = "Record<string, unknown>"))]
    pub capabilities: Option<serde_json::Value>,
}

impl<E: CompletionProvider> LspServer<E> {
    pub fn new(engines: E, config: LspServerConfig) -> Self {
        let mut capabilities = Self::default_capabilities();
        if let Some(c) = config.capabilities {
            deep_merge(&mut capabilities, &c);
        }
        Self {
            engines,
            documents: Arc::new(Mutex::new(HashMap::new())),
            capabilities,
        }
    }

    pub async fn handle_json_rpc(&mut self, req: LspRequest) -> Option<LspResponse> {
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
                let completions = self.engines.complete(uri, &doc);
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

    fn default_capabilities() -> serde_json::Value {
        let mut trigger = vec![];
        trigger.extend(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz.,( "
                .chars()
                .map(|c| c.to_string()),
        );
        json!({
            "completionProvider": { "triggerCharacters": trigger },
            "textDocumentSync": { "openClose": true, "change": 2 },
            "general": { "positionEncodings": ["utf-16"] }
        })
    }
}

fn completion_from_engine(
    doc: &Content, completion: Completion, index: usize,
) -> lsp_types::CompletionItem {
    let start = doc.get_line_col(completion.replace.start);
    let end = doc.get_line_col(completion.replace.end);

    let kind = match &completion.kind {
        CompletionKind::Keyword => lsp_types::CompletionItemKind::KEYWORD,
        CompletionKind::Table => lsp_types::CompletionItemKind::CLASS,
        CompletionKind::Column => lsp_types::CompletionItemKind::FIELD,
        CompletionKind::Schema => lsp_types::CompletionItemKind::MODULE,
        CompletionKind::Function => lsp_types::CompletionItemKind::FUNCTION,
        CompletionKind::Operator => lsp_types::CompletionItemKind::OPERATOR,
    };

    let mut edit = lsp_types::CompletionItem {
        label: completion.label.to_string(),
        insert_text: Some(completion.insert_text.to_string()),
        filter_text: completion.filter_text.map(|s| s.to_string()),
        kind: Some(kind),
        sort_text: Some(format!("{:05}", index)),
        detail: completion.detail.map(|s| s.to_string()),
        ..Default::default()
    };
    match completion.insert_text_format {
        InsertTextFormat::PlainText => {
            edit.text_edit = Some(lsp_types::CompletionTextEdit::Edit(lsp_types::TextEdit {
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
                new_text: completion.insert_text.to_string(),
            }));
        }
        InsertTextFormat::Snippet => {
            edit.insert_text_format = Some(lsp_types::InsertTextFormat::SNIPPET);
            edit.insert_text = Some(completion.insert_text.to_string());
        }
    }
    edit
}

fn deep_merge(a: &mut serde_json::Value, b: &serde_json::Value) {
    match (a, b) {
        (serde_json::Value::Object(a_map), serde_json::Value::Object(b_map)) => {
            for (k, v) in b_map {
                deep_merge(a_map.entry(k.clone()).or_insert(serde_json::Value::Null), v);
            }
        }
        (_, serde_json::Value::Null) => {}
        (a, b) => {
            *a = b.clone();
        }
    }
}
