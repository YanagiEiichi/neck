#[derive(Debug, Clone)]
pub struct Headers(Vec<String>);

impl Headers {}

impl From<Vec<String>> for Headers {
    fn from(value: Vec<String>) -> Self {
        Self(value)
    }
}

impl IntoIterator for Headers {
    type Item = String;

    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Headers {
    /// Get a header value by name (case-insensitive).
    pub fn get_header(&self, key: &str) -> Option<String> {
        let l_key = key.to_lowercase();
        self.0.iter().find_map(|l| {
            let p = l.find(':')?;
            if (&l[..p]).to_lowercase().eq(&l_key) {
                Some(l[p + 1..].trim().to_string())
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

    /// Write data into a Vec<u8>
    pub fn write_bytes(&self, r: &mut Vec<u8>) {
        for i in &self.0 {
            r.extend(i.as_bytes());
            r.push(b'\r');
            r.push(b'\n');
        }
    }
}
