use tokio::io::{AsyncRead, BufReader, self, AsyncWrite, AsyncReadExt, AsyncWriteExt};

use super::Address;

#[derive(Debug, Clone, Default)]
pub struct Host {
  address: Address,
  port: u16
}

impl Host {
  pub async fn read_from<T: AsyncRead + Unpin>(reader: &mut BufReader<T>) -> io::Result<Self> {
    let address = Address::read_from(reader).await?;
    let port = reader.read_u16().await?;
    Ok(Self { address, port })
  }

  pub async fn write_to<T: AsyncWrite + Unpin>(&self, writer: &mut T) -> io::Result<()> {
      self.address.write_to(writer).await?;
      writer.write_u16(self.port).await?;
      Ok(())
  }
}

impl ToString for Host {
    fn to_string(&self) -> String {
      format!("{}:{}", self.address.to_string(), self.port)
    }
}
