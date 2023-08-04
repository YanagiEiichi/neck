use std::error::Error;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt};

use tokio::io::BufReader;

use crate::utils::NeckError;

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
            return Err(NeckError::from("Connection closed by peer"));
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

pub trait HttpCommonBasic {
    fn get_headers(&self) -> &Headers;
    fn get_payload(&self) -> &String;
}

pub type FirstLine = (String, String, String);

#[derive(Debug)]
pub struct HttpProtocol {
    pub first_line: FirstLine,
    pub headers: Headers,
    pub payload: String,
}

impl HttpProtocol {
    pub fn new(first_line: FirstLine, headers: impl Into<Headers>) -> HttpProtocol {
        HttpProtocol {
            first_line,
            headers: headers.into(),
            payload: String::from(""),
        }
    }
    pub async fn read_from<T: AsyncRead>(
        stream: &mut BufReader<T>,
    ) -> Result<HttpProtocol, Box<dyn Error>>
    where
        T: Unpin,
    {
        let lines = read_lines(stream).await?;
        let mut parts = lines[0].trim().splitn(3, ' ');
        let mut hp = HttpProtocol::new(
            (
                String::from(parts.next().unwrap_or("")),
                String::from(parts.next().unwrap_or("")),
                String::from(parts.next().unwrap_or("")),
            ),
            lines[1..].to_vec(),
        );
        hp.payload = hp.read_payload(stream).await?;
        Ok(hp)
    }

    async fn read_payload<T>(&self, stream: &mut BufReader<T>) -> Result<String, Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncRead,
    {
        'a: {
            if let Some(value) = self.headers.get_header("Content-Length") {
                let len = value.parse::<u64>()?;
                if len == 0 {
                    break 'a;
                }
                let mut buf = String::new();
                match stream.take(len).read_to_string(&mut buf).await {
                    Ok(_) => {
                        return Ok(buf);
                    }
                    Err(e) => {
                        return Err(Box::new(e));
                    }
                }
            }
        }
        Ok(String::from(""))
    }
}

impl ToString for HttpProtocol {
    fn to_string(&self) -> String {
        let mut r = String::new();
        r.push_str(&self.first_line.0);
        r.push(' ');
        r.push_str(&self.first_line.1);
        r.push(' ');
        r.push_str(&self.first_line.2);
        r.push_str("\r\n");
        for i in self.headers.clone() {
            r.push_str(&i);
            r.push_str("\r\n");
        }
        r.push_str("\r\n");
        if !self.payload.is_empty() {
            r.push_str(&self.payload);
        }
        r
    }
}
