use std::{io::ErrorKind, net::{Ipv4Addr, Ipv6Addr}};

use tokio::io::{AsyncRead, BufReader, self, AsyncWrite, AsyncWriteExt, AsyncReadExt};

#[derive(Debug, Clone)]
pub enum Socks5Address {
    IPv4(u32),
    Domain(String),
    IPv6(u128),
}

impl Default for Socks5Address {
    fn default() -> Self {
        Socks5Address::IPv4(0)
    }
}

impl Socks5Address {
    pub async fn read_from<T: AsyncRead + Unpin>(reader: &mut BufReader<T>) -> io::Result<Self> {
        let res = match reader.read_u8().await? {
            1 => Socks5Address::IPv4(reader.read_u32().await?),
            4 => Socks5Address::IPv6(reader.read_u128().await?),
            3 => {
                let size = reader.read_u8().await?;
                let mut domain = String::new();
                reader.take(size as u64).read_to_string(&mut domain).await?;
                Socks5Address::Domain(domain)
            }
            _ => return Err(io::Error::new(ErrorKind::Other, "bad protocol")),
        };
        Ok(res)
    }

    pub async fn write_to<T: AsyncWrite + Unpin>(&self, writer: &mut T) -> io::Result<()> {
        match self {
            Socks5Address::IPv4(value) => {
                writer.write_u8(1).await?;
                writer.write_u32(*value).await?;
            }
            Socks5Address::Domain(domain) => {
                writer.write_u8(3).await?;
                writer.write_u8(domain.len() as u8).await?;
                writer.write_all(domain.as_bytes()).await?;
            }
            Socks5Address::IPv6(value) => {
                writer.write_u8(4).await?;
                writer.write_u128(*value).await?;
            }
        }
        Ok(())
    }
}

impl ToString for Socks5Address {
    fn to_string(&self) -> String {
        match self {
            Socks5Address::IPv4(value) => Ipv4Addr::from(*value).to_string(),
            Socks5Address::Domain(domain) => domain.to_string(),
            Socks5Address::IPv6(value) => Ipv6Addr::from(*value).to_string(),
        }
    }
}