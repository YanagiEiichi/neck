use std::{error::Error, ops::Deref};

use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct Headers(Vec<String>);

impl Headers {
    /// Get a header value by name (case-insensitive).
    pub fn get_header(&self, key: &str) -> Option<&str> {
        let l_key = key.to_lowercase();
        self.0.iter().find_map(|l| {
            let p = l.find(':')?;
            if (&l[..p]).to_lowercase().eq(&l_key) {
                Some(l[p + 1..].trim())
            } else {
                None
            }
        })
    }

    /// Remove a header by name (case-insensitive).
    pub fn remove(&mut self, key: &str) -> Option<String> {
        let l_key = key.to_lowercase();
        let index = self.0.iter().position(|l| match l.find(':') {
            Some(p) => (&l[..p]).to_lowercase().eq(&l_key),
            _ => false,
        })?;
        Some(self.0.remove(index))
    }

    // Write data into a Write
    pub async fn write_to<T: AsyncWrite + Unpin>(&self, w: &mut T) -> Result<(), Box<dyn Error>> {
        for i in &self.0 {
            w.write_all(i.as_bytes()).await?;
            w.write_all(b"\r\n").await?;
        }
        Ok(())
    }
}

impl From<Vec<String>> for Headers {
    fn from(value: Vec<String>) -> Self {
        Self(value)
    }
}

impl FromIterator<String> for Headers {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        iter.into_iter().collect::<Vec<String>>().into()
    }
}

impl Deref for Headers {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for Headers {
    type Item = String;

    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
