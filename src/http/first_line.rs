use tokio::io::{self, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct FirstLine {
    // The raw line String, that is trimmed and does not end with CRLF.
    line: String,
    // The location of the first gap.
    gap1: usize,
    // The location of the second gap.
    gap2: usize,
}

impl FirstLine {
    /// Creates a new [`FirstLine`].
    /// For HTTP request, the first line is (method, uri, version).
    /// For HTTP response, the first line is (version, status, text).
    pub fn new(first: &str, second: &str, third: &str) -> Self {
        let gap1 = first.len();
        let gap2 = gap1 + 1 + second.len();
        FirstLine {
            line: format!("{} {} {}", first, second, third),
            gap1,
            gap2,
        }
    }

    /// Returns a reference to the get first of this [`FirstLine`].
    pub fn get_first(&self) -> &str {
        &self.line[..self.gap1]
    }

    /// Returns a reference to the get second of this [`FirstLine`].
    pub fn get_second(&self) -> &str {
        &self.line[self.gap1 + 1..self.gap2]
    }

    /// Returns a reference to the get third of this [`FirstLine`].
    pub fn get_third(&self) -> &str {
        &self.line[self.gap2 + 1..]
    }

    /// Write all data to an AsyncWrite
    pub async fn write_to<T: AsyncWrite + Unpin>(&self, w: &mut T) -> io::Result<()> {
        w.write_all(self.line.as_bytes()).await?;
        w.write_all(b"\r\n").await?;
        Ok(())
    }
}

impl TryFrom<String> for FirstLine {
    type Error = io::Error;

    /// Parse an HTTP first line.
    fn try_from(line: String) -> Result<Self, Self::Error> {
        (|| {
            // Find the first gap, for example:
            //
            // GET /api HTTP/1.1
            //    ↑
            //    here is the first gap
            let gap1 = line.find(' ')?;

            // Find the second gap, for example:
            //
            // GET /api HTTP/1.1
            //         ↑
            //         here is the first gap
            let gap2 = gap1 + 1 + line[gap1 + 1..].find(' ')?;

            Some(Self { line, gap1, gap2 })
        })()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Bad HTTP protocol"))
    }
}
