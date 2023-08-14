use std::error::Error;

use crate::neck::NeckStream;

use super::{connector::Connector, tcp_connector::TcpConnector, tls_connector::TlsConnector};

pub struct ClientContext {
    pub connections: Option<u16>,
    connector: Box<dyn Connector>,
}

impl ClientContext {
    pub fn new(
        addr: String,
        connections: Option<u16>,
        tls_enabled: bool,
        tls_domain: Option<String>,
    ) -> Self {
        Self {
            connections,
            connector: if tls_enabled {
                Box::new(TlsConnector::new(addr, tls_domain))
            } else {
                Box::new(TcpConnector::new(addr))
            },
        }
    }

    pub async fn connect(&self) -> Result<NeckStream, Box<dyn Error>> {
        self.connector.connect().await
    }
}
