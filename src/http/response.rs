use std::error::Error;

use tokio::io::{AsyncRead, BufReader};

use super::{Headers, HttpCommonBasic, HttpProtocol};

pub trait HttpResponse {
    fn get_version(&self) -> &String;
    fn get_raw_status(&self) -> &String;
    fn get_status(&self) -> u16;
    fn get_text(&self) -> &String;
}

impl HttpCommonBasic for HttpResponseBasic {
    fn get_headers(&self) -> &Headers {
        &self.protocol.headers
    }

    fn get_payload(&self) -> &String {
        &self.protocol.payload
    }
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
}

impl ToString for HttpResponseBasic {
    fn to_string(&self) -> String {
        self.protocol.to_string()
    }
}
