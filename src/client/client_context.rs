use std::{error::Error, process::exit};

use crate::neck::NeckStream;

use super::{connector::Connector, tcp_connector::TcpConnector};

#[cfg(feature = "tls")]
use super::tls_connector::TlsConnector;

pub struct ClientContext {
    pub connections: Option<u16>,
    connector: Box<dyn Connector>,
}

impl ClientContext {
    pub fn new(
        addr: String,
        connections: Option<u16>,
        #[allow(unused_variables)] tls_enabled: bool,
        #[allow(unused_variables)] tls_domain: Option<String>,
    ) -> Self {
        Self {
            connections,
            connector: (|| -> Box<dyn Connector> {
                #[cfg(feature = "tls")]
                if tls_enabled {
                    return Box::new(TlsConnector::new(addr, tls_domain));
                }
                if tls_enabled {
                    eprintln!("The '--tls' flag is not supported.");
                    exit(1);
                }
                Box::new(TcpConnector::new(addr))
            })(),
        }
    }

    pub async fn connect(&self) -> Result<NeckStream, Box<dyn Error>> {
        self.connector.connect().await
    }
}
