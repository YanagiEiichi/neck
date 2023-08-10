use std::error::Error;

use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct FirstLine(String, usize, usize);

impl FirstLine {
    /// Creates a new [`FirstLine`].
    pub fn new(first: &str, second: &str, third: &str) -> Self {
        FirstLine(
            format!("{} {} {}", first, second, third),
            first.len(),
            first.len() + 1 + second.len(),
        )
    }

    /// Parse an HTTP first line.
    ///
    /// For example:
    /// raw = "GET /home HTTP/1.1"
    /// gap1 = 3 # Location of the first space character.
    /// offset = gap1 + 1 = 4 # Skip the first space.
    /// gap2 = offset + 5 = 9 # Find space location from "/home .."
    /// Therefore,
    /// get_first returns [..gap1] is "GET"
    /// get_second returns [gap1+1..gap2] is "/home"
    /// get_third returns [gap2+1..] is "HTTP/1.1"
    ///
    pub fn parse(raw: String) -> Option<Self> {
        let gap1 = raw.find(' ')?;
        let offset = gap1 + 1;
        let gap2 = offset + raw[offset..].find(' ')?;
        Some(FirstLine(raw, gap1, gap2))
    }

    /// Returns a reference to the get first of this [`FirstLine`].
    pub fn get_first(&self) -> &str {
        &self.0[..self.1]
    }

    /// Returns a reference to the get second of this [`FirstLine`].
    pub fn get_second(&self) -> &str {
        &self.0[self.1 + 1..self.2]
    }

    /// Returns a reference to the get third of this [`FirstLine`].
    pub fn get_third(&self) -> &str {
        &self.0[self.2 + 1..]
    }

    /// Write all data to an AsyncWrite
    pub async fn write_to<T: AsyncWrite + Unpin>(&self, w: &mut T) -> Result<(), Box<dyn Error>> {
        w.write_all(self.0.as_bytes()).await?;
        w.write_all(b"\r\n").await?;
        Ok(())
    }
}
