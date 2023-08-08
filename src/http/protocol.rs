use std::error::Error;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt};

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
) -> Result<Vec<u8>, Box<dyn Error>>
where
    T: Unpin,
    T: AsyncRead,
{
    let mut buf = Vec::<u8>::new();
    // Get the Content-Length field.
    if let Some(value) = headers.get_header("Content-Length") {
        // Parse it into a integer.
        let len = value.parse::<u64>()?;
        if len > 0 {
            // Read bytes.
            stream.take(len).read_to_end(&mut buf).await?;
        }
    }
    Ok(buf)
}

pub trait HttpCommon {
    /// Get HTTP headers
    fn get_headers(&self) -> &Headers;

    // Convert to HTTP protocol bytes.
    fn to_bytes(&self) -> Vec<u8>;

    /// Get the payload.
    fn get_payload(&self) -> &Vec<u8>;
}

#[derive(Debug)]
pub struct FirstLine(pub String, pub String, pub String);

impl FirstLine {
    fn new(raw: String) -> Result<Self, Box<dyn Error>> {
        if let Some((first, rest)) = raw.split_once(' ') {
            if let Some((second, third)) = rest.split_once(' ') {
                return Ok(FirstLine(
                    String::from(first),
                    String::from(second),
                    String::from(third),
                ));
            }
        }
        HttpError::wrap("Bad HTTP protocol")
    }
    fn write_bytes(&self, u: &mut Vec<u8>) {
        u.extend(self.0.as_bytes());
        u.push(b' ');
        u.extend(self.1.as_bytes());
        u.push(b' ');
        u.extend(self.2.as_bytes());
        u.push(b'\r');
        u.push(b'\n');
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
        // Read HTTP header lines.
        let mut lines = read_lines(stream).await?;

        // Split first line as an iterator.
        let first_line = FirstLine::new(lines.remove(0))?;

        // Create headers (The first line has remove above).
        let headers = Headers::from(lines);

        // Read playload
        let payload = read_payload(stream, &headers).await?;

        Ok(HttpProtocol::new(first_line, headers, payload))
    }
}

impl HttpCommon for HttpProtocol {
    fn get_headers(&self) -> &Headers {
        &self.headers
    }

    fn get_payload(&self) -> &Vec<u8> {
        &self.payload
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut r = Vec::<u8>::new();
        self.first_line.write_bytes(&mut r);
        self.headers.write_bytes(&mut r);
        r.push(b'\r');
        r.push(b'\n');
        if !self.payload.is_empty() {
            r.extend(&self.payload);
        }
        r
    }
}
