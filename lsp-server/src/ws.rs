use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

use querent_lsp::LspJsonCodec;

pub struct WsJsonRpc<Req, Res> {
    socket: WebSocketStream<TcpStream>,
    encoder: LspJsonCodec<Req, Res>,
}

impl<Req, Res> WsJsonRpc<Req, Res>
where
    Req: serde::de::DeserializeOwned,
    Res: serde::Serialize,
{
    pub fn new(websocket: WebSocketStream<TcpStream>, lsp_headers: bool) -> Self {
        Self {
            socket: websocket,
            encoder: LspJsonCodec::new(lsp_headers),
        }
    }

    pub async fn read(&mut self) -> Result<Req, String> {
        use futures_util::StreamExt;

        loop {
            if let Some(res) = self.encoder.decode()? {
                return Ok(res);
            }

            let msg = self
                .socket
                .next()
                .await
                .ok_or_else(|| "WebSocket stream ended".to_string())?
                .map_err(|e| e.to_string())?;
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
                    use futures_util::SinkExt;
                    let _ = self.socket.send(Message::Pong(p)).await;
                }
                Message::Pong(_) => {}
                Message::Close(_) => return Err("Connection closed".into()),
                _ => {}
            }
        }
    }

    pub async fn write(&mut self, response: Res) -> Result<(), String> {
        use futures_util::SinkExt;

        let msg = self.encoder.encode(response)?;

        self.socket
            .send(Message::Text(msg.into()))
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
