use futures::lock::Mutex;
use std::{collections::HashMap, pin::Pin, sync::Arc};

use querent_core::{
    catalog::{CatalogRead, InMemoryCatalog},
    dialect,
    engine::Engine,
};
use querent_lsp::{DocEngineProvider, LspResponse};

mod catalog;
pub use catalog::*;

mod protocol;
pub use protocol::*;

#[derive(Clone)]
pub struct TcpEngines {
    catalogs: Arc<Mutex<HashMap<String, Arc<Engine>>>>,
}

impl DocEngineProvider for TcpEngines {
    fn get(&self, uri: String) -> Pin<Box<dyn Future<Output = Option<Arc<Engine>>> + Send + '_>> {
        Box::pin(async move { Some(self.get(uri).await) })
    }
}

impl TcpEngines {
    pub fn new() -> Self {
        Self {
            catalogs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get(&self, uri: impl Into<String>) -> Arc<Engine> {
        let uri = uri.into();
        let catalogs = self.catalogs.lock().await;

        // Return engine if it exists, otherwise return a default InMemoryCatalog
        catalogs.get(&uri).cloned().unwrap_or_else(|| {
            Arc::new(Engine::new(
                Box::new(InMemoryCatalog::new()),
                dialect::Ansi::default().spec,
            ))
        })
    }

    pub async fn handle(&self, msg: EngineRequest) -> Option<LspResponse> {
        let request_id = match &msg {
            EngineRequest::Set(req) => req.id,
            EngineRequest::Remove(req) => req.id,
        };
        match self.handle_message(msg).await {
            Ok(_) => Some(LspResponse::result(
                request_id,
                EngineResponsePayload::Success,
            )),
            Err(e) => Some(LspResponse::result(
                request_id,
                EngineResponsePayload::Error { message: e },
            )),
        }
    }

    async fn handle_message(&self, msg: EngineRequest) -> Result<(), String> {
        match msg {
            EngineRequest::Set(c) => {
                match c.params.kind {
                    EngineKind::Postgres(postgres) => {
                        let catalog = PostgresConnectedCatalog::from_url(&postgres.uri)
                            .await
                            .map_err(|e| e.to_string())?;

                        let engine = Arc::new(Engine::new(
                            Box::new(CachedCatalog::new(catalog))
                                as Box<dyn CatalogRead + Send + Sync>,
                            dialect::Postgres::default().spec,
                        ));

                        let replaced = self
                            .catalogs
                            .lock()
                            .await
                            .insert(c.params.document_uri.clone(), engine);

                        if let Some(replaced) = replaced {
                            replaced.catalog.close().await;
                        }

                        return Ok(());
                    }
                }

                Ok(())
            }
            EngineRequest::Remove(c) => {
                if let Some(engine) = self.catalogs.lock().await.remove(&c.params.document_uri) {
                    engine.catalog.close().await;
                }
                return Ok(());

                Ok(())
            }
        }
    }
}
