use tokio::net::TcpStream;

use super::connector::{ConnResult, Connector};
use super::neck_url::NeckUrl;

pub struct TcpConnector {
    addr: String,
}

impl TcpConnector {
    pub fn new(url: &NeckUrl) -> Self {
        Self {
            addr: url.get_addr().into(),
        }
    }
}

impl Connector for TcpConnector {
    fn connect(&self) -> ConnResult<'_> {
        Box::pin(async { Ok(TcpStream::connect(&self.addr).await?.into()) })
    }
}
