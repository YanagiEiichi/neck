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
    pub fn get_header(&self, key: &str) -> Option<String> {
        let l_key = key.to_lowercase();
        for line in &self.0 {
            if let Some(p) = line.find(':') {
                if (&line[..p]).to_lowercase().eq(&l_key) {
                    return Some(line[p + 1..].trim().to_string());
                }
            }
        }
        None
    }
}
