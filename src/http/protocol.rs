use std::error::Error;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use tokio::io::BufReader;

use super::error::HttpError;
use super::Headers;

/// Read a group of lines ending with an empty line from a BufReader.
async fn read_lines<T>(stream: &mut BufReader<T>) -> Result<Vec<String>, Box<dyn Error>>
where
    T: Unpin,
    T: AsyncRead,
{
    let mut lines: Vec<String> = Vec::new();
    let mut buf = String::new();
    loop {
        // The `buf` memory space is reused, so it must be cleared each time it is used.
        buf.clear();

        // Normally, the `read` method will wait for any bytes received, so zero bytes read indicate an EOF received.
        if stream.read_line(&mut buf).await? == 0 {
            return HttpError::wrap("Connection closed by peer");
        }

        // The `read_line` retains separator characters such as CR or LF at the end, which should be trimmed.
        let s = buf.trim_end();

        // If an empty line is received.
        if s.is_empty() {
            // And it is the first line of the current context, ignore it and continue reading the next line.
            // otherwise, finish reading and return read lines.
            if lines.is_empty() {
                continue;
            } else {
                break;
            }
        }

        // Now, it is not an empty line, create a copiable String and record it into `lines`.
        lines.push(String::from(s));
    }
    Ok(lines)
}

// Read payload as a Vec<u8>.
async fn read_payload<T>(
    stream: &mut BufReader<T>,
    headers: &Headers,
    buf: &mut Vec<u8>,
) -> Result<u64, Box<dyn Error>>
where
    T: Unpin,
    T: AsyncRead,
{
    // Get the Content-Length field.
    if let Some(value) = headers.get_header("Content-Length") {
        // Parse it into a integer.
        let len = value.parse::<u64>()?;
        if len > 0 {
            // Read bytes.
            stream.take(len).read_to_end(buf).await?;
        }
        return Ok(len);
    }
    return Ok(0);
}

pub trait HttpCommon {
    /// Get HTTP headers
    fn get_headers(&self) -> &Headers;

    /// Get the payload.
    fn get_payload(&self) -> &Vec<u8>;
}

#[derive(Debug)]
pub struct FirstLine(String, usize, usize);

impl FirstLine {
    /// Creates a new [`FirstLine`].
    pub fn new(first: &str, second: &str, third: &str) -> Self {
        FirstLine(
            format!("{} {} {}", first, second, third),
            first.len(),
            first.len() + 1 + second.len(),
        )
    }

    /// Parse an HTTP first line.
    ///
    /// For example:
    /// raw = "GET /home HTTP/1.1"
    /// gap1 = 3 # Location of the first space character.
    /// offset = gap1 + 1 = 4 # Skip the first space.
    /// gap2 = offset + 5 = 9 # Find space location from "/home .."
    /// Therefore,
    /// get_first returns [..gap1] is "GET"
    /// get_second returns [gap1+1..gap2] is "/home"
    /// get_third returns [gap2+1..] is "HTTP/1.1"
    ///
    pub fn parse(raw: String) -> Option<Self> {
        let gap1 = raw.find(' ')?;
        let offset = gap1 + 1;
        let gap2 = offset + raw[offset..].find(' ')?;
        Some(FirstLine(raw, gap1, gap2))
    }

    /// Returns a reference to the get first of this [`FirstLine`].
    pub fn get_first(&self) -> &str {
        &self.0[..self.1]
    }

    /// Returns a reference to the get second of this [`FirstLine`].
    pub fn get_second(&self) -> &str {
        &self.0[self.1 + 1..self.2]
    }

    /// Returns a reference to the get third of this [`FirstLine`].
    pub fn get_third(&self) -> &str {
        &self.0[self.2 + 1..]
    }

    /// Write all data to an AsyncWrite
    pub async fn write_to<T>(&self, w: &mut T) -> Result<(), Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncWrite,
    {
        w.write_all(self.0.as_bytes()).await?;
        w.write_all(b"\r\n").await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct HttpProtocol {
    pub first_line: FirstLine,
    pub headers: Headers,
    pub payload: Vec<u8>,
}

impl HttpProtocol {
    pub fn new(
        first_line: FirstLine,
        headers: impl Into<Headers>,
        payload: Vec<u8>,
    ) -> HttpProtocol {
        HttpProtocol {
            first_line,
            headers: headers.into(),
            payload,
        }
    }
    pub async fn read_from<T>(stream: &mut BufReader<T>) -> Result<HttpProtocol, Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncRead,
    {
        let mut pl = Self::read_header_from(stream).await?;

        // Read playload
        read_payload(stream, &pl.headers, &mut pl.payload).await?;

        Ok(pl)
    }

    pub(crate) async fn read_header_from<T>(
        stream: &mut BufReader<T>,
    ) -> Result<HttpProtocol, Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncRead,
    {
        // Read HTTP header lines.
        let mut lines = read_lines(stream).await?;

        // Try to parse HTTP first line.
        let first_line = match FirstLine::parse(lines.remove(0)) {
            Some(v) => v,
            None => {
                return HttpError::wrap("Bad HTTP Protocol");
            }
        };

        // Create headers (The first line has remove above).
        let headers: Headers = lines.into();

        // Read playload
        let payload = Vec::new();

        Ok(HttpProtocol::new(first_line, headers, payload))
    }

    /// Write all data to an AsyncWrite
    pub async fn write_to<T>(&self, w: &mut T) -> Result<(), Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncWrite,
    {
        self.first_line.write_to(w).await?;
        self.headers.write_to(w).await?;
        w.write_all(b"\r\n").await?;
        if !self.payload.is_empty() {
            w.write_all(&self.payload).await?;
        }
        Ok(())
    }
}

impl HttpCommon for HttpProtocol {
    fn get_headers(&self) -> &Headers {
        &self.headers
    }

    fn get_payload(&self) -> &Vec<u8> {
        &self.payload
    }
}
