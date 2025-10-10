use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

use querent_lsp::{LspRequest, LspResponse, LspServer};
use querent_lsp_engine_tcp::{EngineRequest, TcpEngines};

mod ws;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt::init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "9001".to_string());
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr = format!("{}:{}", host, port);

    let engines = TcpEngines::new();
    start_server(Arc::new(LspServer::new(engines.clone())), engines, addr).await;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Request {
    Engine(EngineRequest),
    Lsp(LspRequest),
}

async fn start_server(server: Arc<LspServer<TcpEngines>>, engines: TcpEngines, addr: String) {
    let tcp_listener = TcpListener::bind(addr.clone())
        .await
        .expect(&format!("Failed to bind to address ws://{}", addr));

    tracing::info!("server listening on ws://{}", addr);

    loop {
        match tcp_listener.accept().await {
            Ok((stream, _addr)) => {
                let server = Arc::clone(&server);
                let engines = engines.clone();
                tokio::spawn(async move {
                    match accept_async(stream).await {
                        Ok(websocket) => {
                            tracing::info!("client connected");
                            handle_client(ws::WsJsonRpc::new(websocket, true), server, engines)
                                .await;
                        }
                        Err(e) => tracing::error!("failed during handshake: {}", e),
                    }
                });
            }
            Err(e) => tracing::error!("failed to accept connection: {}", e),
        }
    }
}

async fn handle_client(
    mut stream: ws::WsJsonRpc<Request, LspResponse>,
    server: Arc<LspServer<TcpEngines>>,
    engines: TcpEngines,
) {
    loop {
        match stream.read().await {
            Ok(request) => {
                tracing::debug!(request = ?request, "received");
                let response = match request {
                    Request::Engine(engine) => engines.handle(engine).await,
                    Request::Lsp(lsp) => server.handle_json_rpc(lsp).await,
                };
                if let Some(response) = response {
                    tracing::debug!(response = ?response, "responding");
                    if let Err(e) = stream.write(response).await {
                        tracing::error!(error = ?e, "failed to send response");
                        break;
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = ?e, "failed to decode request");
                break;
            }
        }
    }
}
