use tokio::net::TcpStream;

use super::connector::{ConnResult, Connector};

pub struct TcpConnector {
    addr: String,
}

impl TcpConnector {
    pub fn new(addr: String) -> Self {
        Self { addr }
    }
}

impl Connector for TcpConnector {
    fn connect(&self) -> ConnResult<'_> {
        Box::pin(async { Ok(TcpStream::connect(&self.addr).await?.into()) })
    }
}
