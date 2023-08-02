use std::error::Error;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt};

use tokio::io::BufReader;

use crate::utils::NeckError;

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

pub trait HttpRequest {
    fn get_method(&self) -> &String;
    fn get_uri(&self) -> &String;
    fn get_version(&self) -> &String;
    fn get_headers(&self) -> &Vec<String>;
    fn get_payload(&self) -> &String;
}

type FirstLine = (String, String, String);

#[derive(Debug)]
pub struct HttpProtocol {
    first_line: FirstLine,
    headers: Vec<String>,
    payload: String,
}

impl HttpProtocol {
    pub fn new(first_line: FirstLine, headers: Vec<String>) -> HttpProtocol {
        HttpProtocol {
            first_line,
            headers,
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

    pub fn get_header(&self, key: &str) -> Option<String> {
        let l_key = key.to_lowercase();
        for line in &self.headers {
            if let Some(p) = line.find(':') {
                if (&line[..p]).to_lowercase().eq(&l_key) {
                    return Some(line[p + 1..].trim().to_string());
                }
            }
        }
        None
    }

    async fn read_payload<T>(&self, stream: &mut BufReader<T>) -> Result<String, Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncRead,
    {
        'a: {
            if let Some(value) = self.get_header("Content-Length") {
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

#[derive(Debug)]
pub struct HttpRequestBasic {
    protocol: HttpProtocol,
}

impl HttpRequestBasic {
    pub fn new(method: &str, uri: &str, version: &str) -> HttpRequestBasic {
        HttpRequestBasic {
            protocol: HttpProtocol::new(
                (
                    String::from(method),
                    String::from(uri),
                    String::from(version),
                ),
                Vec::new(),
            ),
        }
    }

    pub async fn read_from<T>(
        mut stream: &mut BufReader<T>,
    ) -> Result<HttpRequestBasic, Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncRead,
    {
        Ok(HttpRequestBasic {
            protocol: HttpProtocol::read_from(&mut stream).await?,
        })
    }
}

impl HttpRequest for HttpRequestBasic {
    fn get_method(&self) -> &String {
        &self.protocol.first_line.0
    }

    fn get_uri(&self) -> &String {
        &self.protocol.first_line.1
    }

    fn get_version(&self) -> &String {
        &self.protocol.first_line.2
    }

    fn get_headers(&self) -> &Vec<String> {
        &self.protocol.headers
    }

    fn get_payload(&self) -> &String {
        &self.protocol.payload
    }
}

impl ToString for HttpRequestBasic {
    fn to_string(&self) -> String {
        self.protocol.to_string()
    }
}

pub trait HttpResponse {
    fn get_version(&self) -> &String;
    fn get_raw_status(&self) -> &String;
    fn get_status(&self) -> u16;
    fn get_text(&self) -> &String;
    fn get_headers(&self) -> &Vec<String>;
    fn get_payload(&self) -> &String;
}

pub struct HttpResponseBasic {
    protocol: HttpProtocol,
}

impl HttpResponseBasic {
    pub async fn read_from<T>(
        stream: &mut BufReader<T>,
    ) -> Result<HttpResponseBasic, Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncRead,
    {
        let protocol = HttpProtocol::read_from(stream).await?;

        Ok(HttpResponseBasic { protocol })
    }
}

impl HttpResponse for HttpResponseBasic {
    fn get_version(&self) -> &String {
        &self.protocol.first_line.0
    }

    fn get_raw_status(&self) -> &String {
        &self.protocol.first_line.1
    }

    fn get_status(&self) -> u16 {
        self.protocol.first_line.1.parse().unwrap_or_default()
    }

    fn get_text(&self) -> &String {
        &self.protocol.first_line.2
    }

    fn get_headers(&self) -> &Vec<String> {
        &self.protocol.headers
    }

    fn get_payload(&self) -> &String {
        &self.protocol.payload
    }
}

impl ToString for HttpResponseBasic {
    fn to_string(&self) -> String {
        self.protocol.to_string()
    }
}
