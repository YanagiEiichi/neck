use std::{
    borrow::Cow,
    ops::{Range, RangeFrom},
};

use base64::Engine;

pub struct NeckUrl {
    raw: String,
    proto: Range<usize>,
    authorization: Option<String>,
    host: Range<usize>,
    tail: RangeFrom<usize>,
}

impl NeckUrl {
    /// Get the protocol of the URL.
    pub fn get_proto(&self) -> &str {
        &self.raw[self.proto.clone()]
    }

    /// Check if this URL is using HTTPS.
    pub fn is_https(&self) -> bool {
        self.get_proto().eq_ignore_ascii_case("https")
    }

    /// Get the raw authorization header if provided.
    pub fn get_authorization(&self) -> &Option<String> {
        &self.authorization
    }

    /// Get the host from the URL.
    /// Note: It's in the format of host:port, with :port being optional.
    pub fn get_host(&self) -> &str {
        &self.raw[self.host.clone()]
    }

    /// Get the hostname from the URL.
    /// Note: The hostname refers to the domain or IP address without including port.
    #[allow(dead_code)]
    pub fn get_hostname(&self) -> &str {
        let host = self.get_host();
        host.find(':').map_or(host, |p| &host[..p])
    }

    /// Get the addr from the URL.
    /// Note: It's in the format of host:port, if :port being .
    pub fn get_addr(&self) -> Cow<str> {
        let host = self.get_host();
        if host.contains(':') {
            Cow::Borrowed(host)
        } else {
            Cow::Owned(format!(
                "{}:{}",
                host,
                if self.is_https() { 443 } else { 80 }
            ))
        }
    }

    /// Get the tail section of the URL, which includes the path and query string.
    pub fn get_tail(&self) -> &str {
        let a = &self.raw[self.tail.clone()];
        if a.is_empty() {
            "/"
        } else {
            a
        }
    }
}

impl From<String> for NeckUrl {
    fn from(raw: String) -> Self {
        let mut pos = 0;

        // Find the protocol section.
        let proto = if let Some(found) = raw[pos..].find("://") {
            let range = pos..pos + found;
            pos = pos + found + 3;
            range
        } else {
            pos..pos
        };

        // Find the authorization section, which ends with a "@" symbol.
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

        // Find the host section, which typically ends with a slash.
        let host = if let Some(found) = raw[pos..].find("/") {
            let range = pos..pos + found;
            pos = pos + found;
            range
        }
        // If the slash is not found, it indicates all remaining content constitutes the hostname.
        else {
            let range = pos..raw.len();
            pos = raw.len();
            range
        };

        // Save the tail section.
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
