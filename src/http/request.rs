use std::{
    error::Error,
    ops::{Deref, DerefMut},
};

use tokio::io::{AsyncRead, BufReader};

use super::{FirstLine, HttpProtocol};

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

    /// Creates a new [`HttpRequest`].
    pub fn new(method: &str, uri: &str, version: &str) -> Self {
        Self(HttpProtocol::new(
            FirstLine::new(method, uri, version),
            Vec::new(),
            None,
        ))
    }

    pub async fn read_header_from<T>(
        stream: &mut BufReader<T>,
    ) -> Result<HttpRequest, Box<dyn Error>>
    where
        T: Unpin,
        T: AsyncRead,
    {
        Ok(HttpRequest(HttpProtocol::read_header_from(stream).await?))
    }

    /// Returns a reference to the get method of this [`HttpRequest`].
    pub fn get_method(&self) -> &str {
        self.first_line.get_first()
    }

    /// Returns a reference to the get uri of this [`HttpRequest`].
    pub fn get_uri(&self) -> &str {
        self.first_line.get_second()
    }

    /// Returns a reference to the get version of this [`HttpRequest`].
    pub fn get_version(&self) -> &str {
        self.first_line.get_third()
    }
}

impl Deref for HttpRequest {
    type Target = HttpProtocol;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for HttpRequest {
    fn deref_mut(&mut self) -> &mut HttpProtocol {
        &mut self.0
    }
}
