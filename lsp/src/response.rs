use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize)]
pub struct LspResponseEnvelope<R, E> {
    jsonrpc: String,
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<R>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<E>,
}

impl<R, E> LspResponseEnvelope<R, E>
where
    R: Serialize,
    E: Serialize,
{
    pub fn new(id: Option<u64>, result: Option<R>, error: Option<E>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result,
            error,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspResponse(LspResponseEnvelope<serde_json::Value, serde_json::Value>);
impl std::ops::Deref for LspResponse {
    type Target = LspResponseEnvelope<serde_json::Value, serde_json::Value>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for LspResponse {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl LspResponse {
    pub fn result<R>(id: Option<u64>, result: R) -> Self
    where R: Serialize {
        Self(LspResponseEnvelope::new(
            id,
            Some(serde_json::to_value(result).unwrap()),
            None,
        ))
    }

    pub fn error<E>(id: Option<u64>, error: E) -> Self
    where E: Serialize {
        Self(LspResponseEnvelope::new(
            id,
            None,
            Some(serde_json::to_value(error).unwrap()),
        ))
    }
}

impl<R, E> From<LspResponseEnvelope<R, E>> for LspResponse
where
    R: Serialize,
    E: Serialize,
{
    fn from(value: LspResponseEnvelope<R, E>) -> Self {
        Self(LspResponseEnvelope {
            jsonrpc: "2.0".to_string(),
            id: value.id,
            result: value.result.and_then(|r| serde_json::to_value(r).ok()),
            error: value.error.and_then(|e| serde_json::to_value(e).ok()),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspResponseCompletions {
    pub items: Vec<lsp_types::CompletionItem>,
}

impl LspResponseCompletions {
    pub fn new(items: Vec<lsp_types::CompletionItem>) -> Self {
        Self { items }
    }
}
