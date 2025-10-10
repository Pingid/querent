use std::net::TcpStream;
use tungstenite::{Message, WebSocket};

use crate::codec::LspJsonCodec;

pub struct WsJsonRpc<Req, Res> {
    socket: WebSocket<TcpStream>,
    encoder: LspJsonCodec<Req, Res>,
}

impl<Req, Res> WsJsonRpc<Req, Res>
where
    Req: serde::de::DeserializeOwned,
    Res: serde::Serialize,
{
    pub fn new(websocket: WebSocket<TcpStream>, lsp_headers: bool) -> Self {
        Self {
            socket: websocket,
            encoder: LspJsonCodec::new(lsp_headers),
        }
    }

    pub fn read(&mut self) -> Result<Req, String> {
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

    pub fn write(&mut self, response: Res) -> Result<(), String> {
        let msg = self.encoder.encode(response)?;

        self.socket
            .write(Message::Text(msg.into()))
            .map_err(|e| e.to_string())?;

        // Flush to ensure the message is sent
        self.socket.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}
