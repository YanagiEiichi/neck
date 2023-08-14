use tokio::net::TcpStream;

use crate::neck::NeckStream;

use super::connector::{ConnResult, Connector};

pub struct TlsConnector {
    addr: String,
    domain: String,
    connector: tokio_native_tls::TlsConnector,
}

impl TlsConnector {
    pub fn new(addr: String, tls_domain: Option<String>) -> Self {
        // If tls_domain is not set, get the hostname from addr.
        let domain = tls_domain.unwrap_or_else(|| addr.split(':').next().unwrap().to_string());
        // Initialize the TlsConnector
        let connector = native_tls::TlsConnector::new().unwrap().into();
        Self {
            addr,
            domain,
            connector,
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
