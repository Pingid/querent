use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

use tungstenite::{Message, WebSocket, accept};

mod lsp;
mod rpc;

fn main() {
    let server = Arc::new(lsp::LspServer::new());
    let tcp_listener = TcpListener::bind("127.0.0.1:9001").expect("Failed to bind to address");
    println!("SQL LSP WebSocket server listening on ws://127.0.0.1:9001");

    for stream in tcp_listener.incoming() {
        match stream {
            Ok(stream) => {
                let server = Arc::clone(&server);
                thread::spawn(move || match accept(stream) {
                    Ok(websocket) => {
                        println!("New LSP client connected");
                        futures::executor::block_on(handle_client(
                            WsJsonRpc::new(websocket, true),
                            server,
                        ));
                    }
                    Err(e) => eprintln!("Error during websocket handshake: {}", e),
                });
            }
            Err(e) => eprintln!("Error accepting connection: {}", e),
        }
    }
}

async fn handle_client(mut stream: WsJsonRpc, server: Arc<lsp::LspServer>) {
    loop {
        match stream.read() {
            Ok(request) => {
                let response = server.handle_rpc(request).await;
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

pub struct WsJsonRpc {
    socket: WebSocket<TcpStream>,
    encoder: rpc::JsonRpcCodec,
}

impl WsJsonRpc {
    pub fn new(websocket: WebSocket<TcpStream>, lsp_headers: bool) -> Self {
        Self {
            socket: websocket,
            encoder: rpc::JsonRpcCodec::new(lsp_headers),
        }
    }

    pub fn read(&mut self) -> Result<rpc::JsonRequest, String> {
        loop {
            if let Some(res) = self.encoder.decode()? {
                return Ok(res);
            }

            let msg = self.socket.read().map_err(|e| e.to_string())?;
            match msg {
                Message::Text(txt) => self.encoder.buffer(&txt),
                Message::Binary(bin) => {
                    if let Ok(txt) = String::from_utf8(bin.to_vec()) {
                        self.encoder.buffer(&txt);
                    } else {
                        return Err("Non-UTF8 binary message".into());
                    }
                }
                Message::Ping(p) => {
                    let _ = self.socket.write(Message::Pong(p));
                }
                Message::Pong(_) => {}
                Message::Close(_) => return Err("Connection closed".into()),
                _ => {}
            }
        }
    }

    pub fn write(&mut self, response: rpc::ResponseEnvelope) -> Result<(), String> {
        let msg = self.encoder.encode(response)?;

        self.socket
            .write(Message::Text(msg.into()))
            .map_err(|e| e.to_string())?;

        // Flush to ensure the message is sent
        self.socket.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}
