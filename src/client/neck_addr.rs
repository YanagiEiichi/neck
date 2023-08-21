use std::ops::{Range, RangeFrom};

use base64::Engine;

pub struct NeckAddr {
    raw: String,
    proto: Range<usize>,
    authorization: Option<String>,
    host: Range<usize>,
    tail: RangeFrom<usize>,
}

impl NeckAddr {
    pub fn get_proto(&self) -> &str {
        &self.raw[self.proto.clone()]
    }

    pub fn get_authorization(&self) -> &Option<String> {
        &self.authorization
    }

    pub fn get_host(&self) -> &str {
        &self.raw[self.host.clone()]
    }

    pub fn get_tail(&self) -> &str {
        let a = &self.raw[self.tail.clone()];
        if a.is_empty() {
            "/"
        } else {
            a
        }
    }
}

impl From<String> for NeckAddr {
    fn from(raw: String) -> Self {
        let mut pos = 0;

        let proto = if let Some(found) = raw[pos..].find("://") {
            let range = pos..pos + found;
            pos = pos + found + 3;
            range
        } else {
            pos..pos
        };

        let authorization = {
            if let Some(found) = raw[pos..].find("@") {
                let value =
                    base64::engine::general_purpose::STANDARD.encode(&raw[pos..pos + found]);
                pos = pos + found + 1;
                Some(format!("Authorization: Basic {}", value))
            } else {
                None
            }
        };

        let host = if let Some(found) = raw[pos..].find("/") {
            let range = pos..pos + found;
            pos = pos + found;
            range
        } else {
            let range = pos..raw.len();
            pos = raw.len();
            range
        };

        let tail = pos..;

        Self {
            raw,
            proto,
            authorization,
            host,
            tail,
        }
    }
}
