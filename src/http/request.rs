use std::error::Error;

use tokio::io::{AsyncRead, BufReader};

use super::{FirstLine, Headers, HttpCommon, HttpProtocol};

#[derive(Debug)]
pub struct HttpRequest {
    protocol: HttpProtocol,
}

impl HttpRequest {
    pub fn new(method: &str, uri: &str, version: &str, headers: impl Into<Headers>) -> HttpRequest {
        HttpRequest {
            protocol: HttpProtocol::new(
                FirstLine(
                    String::from(method),
                    String::from(uri),
                    String::from(version),
                ),
                headers.into(),
                Vec::new(),
            ),
        }
    }

    pub async fn read_from<T>(mut stream: &mut BufReader<T>) -> Result<HttpRequest, Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncRead,
    {
        Ok(HttpRequest {
            protocol: HttpProtocol::read_from(&mut stream).await?,
        })
    }

    pub fn get_method(&self) -> &String {
        &self.protocol.first_line.0
    }

    pub fn get_uri(&self) -> &String {
        &self.protocol.first_line.1
    }

    pub fn get_version(&self) -> &String {
        &self.protocol.first_line.2
    }
}

impl HttpCommon for HttpRequest {
    fn get_headers(&self) -> &Headers {
        &self.protocol.headers
    }

    fn get_payload(&self) -> &Vec<u8> {
        self.protocol.get_payload()
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.protocol.to_bytes()
    }
}
