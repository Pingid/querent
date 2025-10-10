use futures::lock::Mutex;
use std::{collections::HashMap, sync::Arc};

use querent_catalogs::{postgres::PostgresConnectedCatalog, util::CachedCatalog};
use querent_core::{
    catalog::{CatalogRead, InMemoryCatalog},
    dialect,
    engine::Engine,
};

mod proto;
pub use proto::*;

use crate::lsp_protocol::LspJsonResponse;

#[derive(Clone)]
pub struct Engines {
    catalogs: Arc<Mutex<HashMap<String, Arc<Engine>>>>,
}

impl Engines {
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

    pub async fn handle(&self, msg: EngineRequest) -> Option<LspJsonResponse> {
        let request_id = match &msg {
            EngineRequest::Set(req) => req.id,
            EngineRequest::Remove(req) => req.id,
        };
        match self.handle_message(msg).await {
            Ok(_) => Some(LspJsonResponse::result(
                request_id,
                serde_json::to_value(EngineResponsePayload::Success).unwrap(),
            )),
            Err(e) => Some(LspJsonResponse::result(
                request_id,
                serde_json::to_value(EngineResponsePayload::Error { message: e }).unwrap(),
            )),
        }
    }

    async fn handle_message(&self, msg: EngineRequest) -> Result<(), String> {
        match msg {
            EngineRequest::Set(c) => {
                if let Some(params) = c.params {
                    match params.kind {
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
                                .insert(params.document_uri.clone(), engine);

                            if let Some(replaced) = replaced {
                                replaced.catalog.close().await;
                            }

                            return Ok(());
                        }
                    }
                }
                Ok(())
            }
            EngineRequest::Remove(c) => {
                if let Some(params) = c.params {
                    if let Some(engine) = self.catalogs.lock().await.remove(&params.document_uri) {
                        engine.catalog.close().await;
                    }
                    return Ok(());
                }
                Ok(())
            }
        }
    }
}
