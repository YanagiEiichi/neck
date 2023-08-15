use std::ops::{Deref, DerefMut};

use super::{FirstLine, HttpProtocol};

#[derive(Debug)]
pub struct HttpResponse(HttpProtocol);

impl HttpResponse {
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

impl From<HttpProtocol> for HttpResponse {
    fn from(protocol: HttpProtocol) -> Self {
        Self(protocol)
    }
}
