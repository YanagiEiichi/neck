use crate::utils::connect;

use super::{
    super::neck_url::NeckUrl,
    {ConnResult, Connector},
};

pub struct TlsConnector {
    addr: String,
    domain: String,
    connector: tokio_native_tls::TlsConnector,
}

impl TlsConnector {
    pub fn new(url: &NeckUrl, tls_domain: Option<String>) -> Self {
        Self {
            addr: url.get_addr().into(),
            // If tls_domain is not set, get the hostname from URL.
            domain: tls_domain.unwrap_or_else(|| url.get_hostname().into()),
            // Initialize the TlsConnector
            connector: native_tls::TlsConnector::new().unwrap().into(),
        }
    }
}

impl Connector for TlsConnector {
    fn connect(&self) -> ConnResult<'_> {
        Box::pin(async {
            // Attempt to connect Neck Server.
            let tcp_stream = connect(&self.addr).await?;

            // Wrap the TcpStream with TlsSteram.
            let tls_stream = self.connector.connect(&self.domain, tcp_stream).await?;

            // Wrap the TlsSteram stream with NeckStream
            Ok(tls_stream.into())
        })
    }
}
