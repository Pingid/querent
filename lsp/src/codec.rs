use std::marker::PhantomData;

pub struct LspJsonCodec<Req, Res> {
    buffer: String,
    lsp_headers: bool,
    _req: PhantomData<Req>,
    _res: PhantomData<Res>,
}

impl<Req, Res> LspJsonCodec<Req, Res>
where
    Req: serde::de::DeserializeOwned,
    Res: serde::Serialize,
{
    pub fn new(lsp_headers: bool) -> Self {
        Self {
            buffer: String::new(),
            lsp_headers,
            _req: PhantomData,
            _res: PhantomData,
        }
    }

    pub fn buffer(&mut self, s: &str) {
        self.buffer.push_str(s);
    }

    pub fn encode(&self, response: Res) -> Result<String, String> {
        let json = serde_json::to_string(&response).map_err(|e| e.to_string())?;
        let msg = match self.lsp_headers {
            true => format!("Content-Length: {}\r\n\r\n{}", json.len(), json),
            false => json,
        };
        Ok(msg)
    }

    pub fn decode(&mut self) -> Result<Option<Req>, String> {
        match self.try_extract_message()? {
            Some(s) => match serde_json::from_str::<Req>(&s) {
                Ok(req) => Ok(Some(req)),
                Err(_) => Err(format!("Error decoding request: {}", s)),
            },
            None => Ok(None),
        }
    }

    fn try_extract_message(&mut self) -> Result<Option<String>, String> {
        let buf = self.buffer.as_str();

        if !self.lsp_headers {
            let json_text = buf.to_string();
            self.buffer = buf[buf.len()..].trim_start().to_string();
            return Ok(Some(json_text));
        }

        // must start with "Content-Length"
        if !buf.starts_with("Content-Length:") {
            return Ok(None);
        }

        // find header terminator
        let header_end = match buf.find("\r\n\r\n") {
            Some(pos) => pos,
            None => return Ok(None),
        };

        // parse content length
        let header = &buf[..header_end];
        let content_length = header
            .lines()
            .find_map(|l| l.strip_prefix("Content-Length:"))
            .and_then(|v| v.trim().parse::<usize>().ok())
            .ok_or_else(|| "Invalid Content-Length header".to_string())?;

        let start = header_end + 4;
        let end = start + content_length;
        if buf.len() < end {
            return Ok(None); // incomplete payload
        }

        // extract and trim buffer
        let json_text = buf[start..end].to_string();
        self.buffer = buf[end..].trim_start().to_string();
        Ok(Some(json_text))
    }
}
