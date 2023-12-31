use crate::utils::connect;

use super::super::neck_url::NeckUrl;
use super::{ConnResult, Connector};

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
        Box::pin(async { Ok(connect(&self.addr).await?.into()) })
    }
}
