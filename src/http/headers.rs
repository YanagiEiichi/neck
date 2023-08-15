use std::{
    error::Error,
    ops::{Deref, DerefMut},
};

use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct HeaderRow(
    // The raw header row string (without CRLF).
    String,
    // Location of the first colon.
    usize,
);

impl HeaderRow {
    /// Creates a new [`HeaderRow`].
    pub fn new(raw: String, colon: usize) -> Self {
        Self(raw, colon)
    }

    /// Creates a new [`HeaderRow`].
    pub fn new_with_kv(name: &str, value: &str) -> Self {
        Self::new(format!("{}: {}", name, value), name.len())
    }

    /// Get header name
    pub fn get_name(&self) -> &str {
        &self.0[..self.1]
    }

    /// Get header value
    pub fn get_value(&self) -> &str {
        // Some spaces may be places following the colon, so `trim_start` is needed here.
        &self.0[self.1 + 1..].trim_start()
    }

    /// Get header value
    pub fn set_value(&mut self, value: &str) {
        self.0 = format!("{}: {}", self.get_name(), value);
    }

    /// Compare the name (case-insensitive).
    pub fn eq_name(&self, name: &str) -> bool {
        self.get_name().eq_ignore_ascii_case(name)
    }

    /// Write the data into an AsyncWrite (a CRLF will be appended at the end).
    pub async fn write_to<T: AsyncWrite + Unpin>(&self, w: &mut T) -> Result<(), Box<dyn Error>> {
        w.write_all(self.0.as_bytes()).await?;
        w.write_all(b"\r\n").await?;
        Ok(())
    }
}

impl PartialEq for HeaderRow {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0) && self.1 == other.1
    }
}

impl Deref for HeaderRow {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for HeaderRow {
    fn from(raw: String) -> Self {
        // Find the first colon.
        match raw.find(':') {
            // If it exists, save it.
            Some(colon) => Self(raw, colon),
            // If no colon in the string, append a colon.
            None => {
                let colon = raw.len();
                Self(raw + ":", colon)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Headers(Vec<HeaderRow>);

impl Headers {
    /// Get a header value by name (case-insensitive).
    /// TODO: Rename to get_header_value.
    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|l| l.eq_name(name))
            .map(|v| v.get_value())
    }

    /// Set a header value by name (case-insensitive).
    #[allow(unused)]
    pub fn set_header(&mut self, name: &str, value: &str) {
        let found = self.0.iter_mut().find(|l| l.eq_name(name));
        match found {
            Some(row) => row.set_value(value),
            None => self.push(HeaderRow::new(format!("{}: {}", name, value), name.len())),
        };
    }

    /// Remove a header by name (case-insensitive).
    #[allow(unused)]
    pub fn remove(&mut self, name: &str) -> Option<HeaderRow> {
        let index = self.0.iter().position(|l| l.eq_name(name))?;
        Some(self.0.remove(index))
    }

    /// Write the data into an AsyncWrite (a CRLF will be appended at the end of each item).
    pub async fn write_to<T: AsyncWrite + Unpin>(&self, w: &mut T) -> Result<(), Box<dyn Error>> {
        for i in &self.0 {
            i.write_to(w).await?;
        }
        Ok(())
    }
}

impl From<Vec<String>> for Headers {
    fn from(value: Vec<String>) -> Self {
        Self(value.into_iter().map(|v| HeaderRow::from(v)).collect())
    }
}

impl FromIterator<String> for Headers {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        iter.into_iter().collect::<Vec<String>>().into()
    }
}

impl Deref for Headers {
    type Target = Vec<HeaderRow>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Headers {
    fn deref_mut(&mut self) -> &mut Vec<HeaderRow> {
        &mut self.0
    }
}

impl IntoIterator for Headers {
    type Item = HeaderRow;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
