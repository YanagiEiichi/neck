use std::error::Error;

use tokio::net::TcpStream;

use crate::neck::NeckStream;

pub struct ClientContext {
    pub addr: String,
    pub connections: Option<u16>,
    tls: Option<(tokio_native_tls::TlsConnector, String)>,
}

impl ClientContext {
    pub fn new(
        addr: String,
        connections: Option<u16>,
        tls_enabled: bool,
        tls_domain: Option<String>,
    ) -> Self {
        // Create tls context only when tls is enabled.
        let tls = tls_enabled.then(|| {
            (
                // Initialize the TlsConnector
                native_tls::TlsConnector::new().unwrap().into(),
                // If tls_domain is not set, get the hostname from addr.
                tls_domain.unwrap_or_else(|| addr.split(':').next().unwrap().to_string()),
            )
        });

        Self {
            addr,
            connections,
            tls,
        }
    }

    pub async fn connect(&self) -> Result<NeckStream, Box<dyn Error>> {
        // Attempt to connect Neck Server.
        let tcp_stream = TcpStream::connect(&self.addr).await?;

        // Get addresses pairs.
        let peer_addr = tcp_stream.peer_addr().unwrap();
        let local_addr = tcp_stream.local_addr().unwrap();

        // Connect NeckServer (may over TLS)
        let stream = match self.tls.as_ref() {
            // If tls is enabled.
            Some((connector, domain)) => {
                // Wrap the TcpStream with TlsSteram.
                let tls_stream = connector.connect(domain, tcp_stream).await?;
                // Wrap the TlsSteram stream with NeckStream
                NeckStream::new(peer_addr, local_addr, tls_stream)
            }
            // Otherwise, the tls is not enabled.
            None => {
                // Wrap the TcpSteram stream with NeckStream
                NeckStream::new(peer_addr, local_addr, tcp_stream)
            }
        };

        Ok(stream)
    }
}
