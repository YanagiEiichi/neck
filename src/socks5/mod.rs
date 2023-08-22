use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
mod address;
mod greeting;
mod host;

use address::*;
use host::*;

pub use greeting::*;

#[derive(Debug, Clone)]
pub struct Socks5Message {
    pub action: u8,
    pub host: Host,
}

impl Socks5Message {
    pub async fn read_from<T: AsyncRead + Unpin>(reader: &mut BufReader<T>) -> io::Result<Self> {
        reader.read_u8().await?; // version
        let action = reader.read_u8().await?;
        reader.read_u8().await?; // rsv
        let host = Host::read_from(reader).await?;
        Ok(Self { action, host })
    }

    pub async fn write_to<T: AsyncWrite + Unpin>(&self, writer: &mut T) -> io::Result<()> {
        writer.write_u8(5).await?; // version
        writer.write_u8(self.action).await?;
        writer.write_u8(0).await?; // rsv
        self.host.write_to(writer).await?;
        Ok(())
    }

    pub fn set_action(&mut self, action: u8) -> &mut Self {
        self.action = action;
        self
    }
}
