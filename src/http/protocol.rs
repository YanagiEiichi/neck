use std::borrow::Cow;

use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use tokio::io::BufReader;

use super::utils::read_lines;
use super::{FirstLine, HeaderRow, Headers};

// Read payload as a Vec<u8>.
async fn read_payload<T: AsyncRead + Unpin>(
    stream: &mut BufReader<T>,
    headers: &Headers,
) -> io::Result<Vec<u8>> {
    let mut buf = Vec::<u8>::new();
    // Get the Content-Length field.
    if let Some(value) = headers.get_header("Content-Length") {
        // Parse it into a integer.
        let len = match value.parse::<u64>() {
            Ok(it) => it,
            Err(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Bad Content-Length",
            ))?,
        };
        // Check body size limit.
        if len > 16 * 1024u64 {
            Err(io::Error::new(io::ErrorKind::OutOfMemory, "Body too long"))?
        }
        if len > 0 {
            // Read bytes.
            stream.take(len).read_to_end(&mut buf).await?;
        }
    }
    return Ok(buf);
}

pub trait HttpCommon {
    /// Get HTTP headers
    fn get_headers(&self) -> &Headers;

    /// Get the payload.
    fn get_payload(&self) -> &Option<Vec<u8>>;
}

#[derive(Debug)]
pub struct HttpProtocol {
    pub first_line: FirstLine,
    pub headers: Headers,
    // TODO: Move payload out as another struct.
    pub payload: Option<Vec<u8>>,
}

impl HttpProtocol {
    pub fn new(
        first_line: FirstLine,
        headers: impl Into<Headers>,
        payload: Option<Vec<u8>>,
    ) -> HttpProtocol {
        HttpProtocol {
            first_line,
            headers: headers.into(),
            payload,
        }
    }
    pub async fn read_from<T: AsyncRead + Unpin>(
        stream: &mut BufReader<T>,
    ) -> io::Result<HttpProtocol> {
        let mut pl = Self::read_header_from(stream).await?;

        // Read playload
        pl.payload = Some(read_payload(stream, &pl.headers).await?);

        Ok(pl)
    }

    pub(crate) async fn read_header_from<T: AsyncRead + Unpin>(
        stream: &mut BufReader<T>,
    ) -> io::Result<HttpProtocol> {
        // Maximun allowed size for an HTTP header.
        let mut budget = 16 * 1024usize;

        // Read HTTP header lines.
        let mut lines = read_lines(stream, &mut budget).await?;

        // Try to parse HTTP first line.
        let first_line: FirstLine = lines.remove(0).try_into()?;

        // Create headers (The first line has remove above).
        let headers: Headers = lines.into();

        Ok(HttpProtocol::new(first_line, headers, None))
    }

    /// Add a request header with raw string.
    pub fn add_header(&mut self, kv: impl Into<Cow<'static, str>>) -> &mut Self {
        self.headers.push(kv.into().into_owned().into());
        self
    }

    /// Add a request header with name and value.
    pub fn add_header_kv(&mut self, name: &str, value: &str) -> &mut Self {
        self.headers.push(HeaderRow::new_with_kv(name, value));
        self
    }

    /// Add a request header with Option<String>.
    pub(crate) fn add_header_option(&mut self, option: &Option<String>) -> &Self {
        if let Some(kv) = option {
            self.headers.push(kv.to_string().into());
        }
        self
    }

    /// Push data to payload.
    pub fn add_payload(&mut self, bytes: &[u8]) -> &mut Self {
        if let Some(payload) = self.payload.as_mut() {
            payload.extend(bytes);
        } else {
            self.payload = Some(Vec::from(bytes));
        }
        self
    }

    /// Write all data to an AsyncWrite
    pub async fn write_to<T: AsyncWrite + Unpin>(&self, w: &mut T) -> io::Result<()> {
        self.first_line.write_to(w).await?;

        match self.payload.as_ref() {
            // Recalculate the actual value of Content-Length.
            Some(payload) => {
                let mut content_type_sent = false;
                for h in self.headers.iter() {
                    // Update flag if Content-Type has sent.
                    if h.eq_name("Content-Type") {
                        content_type_sent = true;
                    }

                    // If Ignore unsafe Content-Length.
                    // This header will be recalculated set later.
                    if h.eq_name("Content-Length") {
                        continue;
                    }

                    h.write_to(w).await?;
                }

                // Set the default Content-Type to text/plain.
                if !content_type_sent {
                    w.write_all(b"Content-Type: text/plain\r\n").await?;
                }

                // Write the Content-Length header that is calculated based on the actual payload.
                w.write_all(format!("Content-Length: {}\r\n", payload.len()).as_bytes())
                    .await?;
            }
            // Pass all headers through.
            None => {
                self.headers.write_to(w).await?;
            }
        }

        // All headers have been sent.
        w.write_all(b"\r\n").await?;

        // Send the payload if it exists.
        if let Some(payload) = self.payload.as_ref() {
            if payload.len() > 0 {
                w.write_all(payload).await?;
            }
        }

        Ok(())
    }
}

impl HttpCommon for HttpProtocol {
    fn get_headers(&self) -> &Headers {
        &self.headers
    }

    fn get_payload(&self) -> &Option<Vec<u8>> {
        &self.payload
    }
}
