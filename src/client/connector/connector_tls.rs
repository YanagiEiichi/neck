use tokio::net::TcpStream;

use crate::utils::NeckStream;

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
            let tcp_stream = TcpStream::connect(&self.addr).await?;

            // Get addresses pairs.
            let peer_addr = tcp_stream.peer_addr().unwrap();
            let local_addr = tcp_stream.local_addr().unwrap();

            // Wrap the TcpStream with TlsSteram.
            let tls_stream = self.connector.connect(&self.domain, tcp_stream).await?;

            // Wrap the TlsSteram stream with NeckStream
            Ok(NeckStream::new(peer_addr, local_addr, tls_stream))
        })
    }
}
