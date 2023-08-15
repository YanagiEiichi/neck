use std::{
    error::Error,
    ops::{Deref, DerefMut},
};

use tokio::io::{AsyncRead, BufReader};

use super::{FirstLine, HttpProtocol};

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

    /// Creates a new [`HttpResponse`].
    pub fn new(status: u16, text: &str, version: &str) -> Self {
        Self(HttpProtocol::new(
            FirstLine::new(version, &status.to_string(), text),
            Vec::new(),
            None,
        ))
    }

    /// Returns a reference to the get version of this [`HttpResponse`].
    #[allow(dead_code)]
    pub fn get_version(&self) -> &str {
        self.first_line.get_first()
    }

    /// Returns the get status of this [`HttpResponse`].
    pub fn get_status(&self) -> u16 {
        self.first_line.get_second().parse().unwrap_or_default()
    }

    /// Returns a reference to the get text of this [`HttpResponse`].
    #[allow(dead_code)]
    pub fn get_text(&self) -> &str {
        self.first_line.get_third()
    }
}

impl Deref for HttpResponse {
    type Target = HttpProtocol;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HttpResponse {
    fn deref_mut(&mut self) -> &mut HttpProtocol {
        &mut self.0
    }
}
