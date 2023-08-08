use std::{error::Error, ops::Deref};

use tokio::io::{AsyncRead, BufReader};

use super::HttpProtocol;

#[derive(Debug)]
pub struct HttpRequest(HttpProtocol);

impl HttpRequest {
    pub async fn read_from<T>(stream: &mut BufReader<T>) -> Result<HttpRequest, Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncRead,
    {
        Ok(HttpRequest(HttpProtocol::read_from(stream).await?))
    }

    /// Returns a reference to the get method of this [`HttpRequest`].
    pub fn get_method(&self) -> &String {
        &self.first_line.0
    }

    /// Returns a reference to the get uri of this [`HttpRequest`].
    pub fn get_uri(&self) -> &String {
        &self.first_line.1
    }

    /// Returns a reference to the get version of this [`HttpRequest`].
    pub fn get_version(&self) -> &String {
        &self.first_line.2
    }
}

impl Deref for HttpRequest {
    type Target = HttpProtocol;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
