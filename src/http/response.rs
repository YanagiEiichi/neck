use std::error::Error;

use tokio::io::{AsyncRead, BufReader};

use super::{Headers, HttpCommonBasic, HttpProtocol};

pub struct HttpResponse {
    protocol: HttpProtocol,
}

impl HttpResponse {
    pub async fn read_from<T>(stream: &mut BufReader<T>) -> Result<HttpResponse, Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncRead,
    {
        Ok(HttpResponse {
            protocol: HttpProtocol::read_from(stream).await?,
        })
    }

    /// Returns a reference to the get version of this [`HttpResponse`].
    #[allow(dead_code)]
    pub fn get_version(&self) -> &String {
        &self.protocol.first_line.0
    }

    /// Returns the get status of this [`HttpResponse`].
    pub fn get_status(&self) -> u16 {
        self.protocol.first_line.1.parse().unwrap_or_default()
    }

    /// Returns a reference to the get text of this [`HttpResponse`].
    #[allow(dead_code)]
    pub fn get_text(&self) -> &String {
        &self.protocol.first_line.2
    }
}

impl HttpCommonBasic for HttpResponse {
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
