use std::error::Error;

use tokio::io::{AsyncRead, BufReader};

use super::{Headers, HttpCommonBasic, HttpProtocol};

pub trait HttpRequest {
    fn get_method(&self) -> &String;
    fn get_uri(&self) -> &String;
    fn get_version(&self) -> &String;
}

#[derive(Debug)]
pub struct HttpRequestBasic {
    protocol: HttpProtocol,
}

impl HttpRequestBasic {
    pub fn new(
        method: &str,
        uri: &str,
        version: &str,
        headers: impl Into<Headers>,
    ) -> HttpRequestBasic {
        HttpRequestBasic {
            protocol: HttpProtocol::new(
                (
                    String::from(method),
                    String::from(uri),
                    String::from(version),
                ),
                headers.into(),
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
}

impl HttpCommonBasic for HttpRequestBasic {
    fn get_headers(&self) -> &Headers {
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
