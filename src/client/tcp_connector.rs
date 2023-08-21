use tokio::net::TcpStream;

use super::connector::{ConnResult, Connector};
use super::neck_addr::NeckAddr;

pub struct TcpConnector {
    host: String,
}

impl TcpConnector {
    pub fn new(addr: &NeckAddr) -> Self {
        Self {
            host: addr.get_host().to_string(),
        }
    }
}

impl Connector for TcpConnector {
    fn connect(&self) -> ConnResult<'_> {
        Box::pin(async { Ok(TcpStream::connect(&self.host).await?.into()) })
    }
}
