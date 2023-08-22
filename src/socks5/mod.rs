use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
mod greeting;
mod address;

pub use greeting::*;
pub use address::*;


#[derive(Debug, Clone)]
pub struct Sock5Connection {
    pub version: u8,
    pub action: u8,
    pub dst_addr: Socks5Address,
    pub dst_port: u16,
}

impl Sock5Connection {
    pub async fn read_from<T: AsyncRead + Unpin>(reader: &mut BufReader<T>) -> io::Result<Self> {
        let version = reader.read_u8().await?;
        let action = reader.read_u8().await?;
        reader.read_u8().await?; // rsv
        let dst_addr = Socks5Address::read_from(reader).await?;
        let dst_port = reader.read_u16().await?;
        Ok(Self {
            version,
            action,
            dst_addr,
            dst_port,
        })
    }

    pub async fn write_to<T: AsyncWrite + Unpin>(&self, writer: &mut T) -> io::Result<()> {
        writer.write_u8(self.version).await?;
        writer.write_u8(self.action).await?;
        writer.write_u8(0).await?; // rsv
        self.dst_addr.write_to(writer).await?;
        writer.write_u16(self.dst_port).await?;
        Ok(())
    }

    pub fn to_addr(&self) -> String {
        format!("{}:{}", self.dst_addr.to_string(), self.dst_port)
    }

    pub fn new(action: u8) -> Sock5Connection {
        Self {
            version: 5,
            action,
            dst_addr: Socks5Address::default(),
            dst_port: 0,
        }
    }
}
