use std::net::TcpListener;
use std::sync::Arc;
use tungstenite::accept;

mod codec;
mod engines;
mod lsp_protocol;
mod proto;
mod server;
mod ws;

use crate::proto::{ProtoRequest, ProtoResponse};
use crate::ws::WsJsonRpc;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let server = Arc::new(server::LspServer::new());
    let tcp_listener = TcpListener::bind("127.0.0.1:9001").expect("Failed to bind to address");
    println!("SQL LSP WebSocket server listening on ws://127.0.0.1:9001");

    for stream in tcp_listener.incoming() {
        match stream {
            Ok(stream) => {
                let server = Arc::clone(&server);
                tokio::spawn(async move {
                    match accept(stream) {
                        Ok(websocket) => {
                            println!("New LSP client connected");
                            handle_client(WsJsonRpc::new(websocket, true), server).await;
                        }
                        Err(e) => eprintln!("Error during websocket handshake: {}", e),
                    }
                });
            }
            Err(e) => eprintln!("Error accepting connection: {}", e),
        }
    }
}

async fn handle_client(
    mut stream: WsJsonRpc<ProtoRequest, ProtoResponse>,
    server: Arc<server::LspServer>,
) {
    loop {
        match stream.read() {
            Ok(request) => {
                let response = server.handle_proto_request(request).await;
                if let Some(response) = response {
                    if let Err(e) = stream.write(response) {
                        eprintln!("Error sending response: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error decoding request: {}", e);
                break;
            }
        }
    }
}
