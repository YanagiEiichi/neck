use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

#[derive(Debug)]
pub struct ClientGreeting {
    pub ver: u8,
    pub auth_list: Vec<u8>,
}

impl ClientGreeting {
    pub async fn read_from<T: AsyncRead + Unpin>(reader: &mut BufReader<T>) -> io::Result<Self> {
        let ver = reader.read_u8().await?;
        let size = reader.read_u8().await?;
        let mut auth_list = Vec::new();
        reader.take(size as u64).read_to_end(&mut auth_list).await?;
        Ok(Self { ver, auth_list })
    }
}

#[derive(Debug)]
pub struct ServerChoice {
    pub ver: u8,
    pub cauth: u8,
}

impl ServerChoice {
    pub fn new(ver: u8, cauth: u8) -> Self {
        Self { ver, cauth }
    }

    pub async fn write_to<T: AsyncWrite + Unpin>(&self, writer: &mut T) -> io::Result<()> {
        writer.write_all(&[self.ver, self.cauth]).await?;
        Ok(())
    }
}