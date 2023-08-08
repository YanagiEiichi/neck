use std::{error::Error, ops::Deref};

use tokio::io::{AsyncRead, BufReader};

use super::HttpProtocol;

#[derive(Debug)]
pub struct HttpResponse(HttpProtocol);

impl HttpResponse {
    pub async fn read_from<T>(stream: &mut BufReader<T>) -> Result<HttpResponse, Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncRead,
    {
        Ok(HttpResponse(HttpProtocol::read_from(stream).await?))
    }

    /// Returns a reference to the get version of this [`HttpResponse`].
    #[allow(dead_code)]
    pub fn get_version(&self) -> &String {
        &self.first_line.0
    }

    /// Returns the get status of this [`HttpResponse`].
    pub fn get_status(&self) -> u16 {
        self.first_line.1.parse().unwrap_or_default()
    }

    /// Returns a reference to the get text of this [`HttpResponse`].
    #[allow(dead_code)]
    pub fn get_text(&self) -> &String {
        &self.first_line.2
    }
}

impl Deref for HttpResponse {
    type Target = HttpProtocol;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
