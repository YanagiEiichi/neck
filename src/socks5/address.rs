use std::{
    fmt::Display,
    io::ErrorKind,
    net::{Ipv4Addr, Ipv6Addr},
};

use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

/// https://datatracker.ietf.org/doc/html/rfc1928#section-5
#[derive(Debug, Clone)]
pub enum Address {
    IPv4(u32),
    Domain(String),
    IPv6(u128),
}

impl Address {
    pub async fn read_from<T: AsyncRead + Unpin>(reader: &mut BufReader<T>) -> io::Result<Self> {
        let res = match reader.read_u8().await? {
            1 => Address::IPv4(reader.read_u32().await?),
            3 => {
                let size = reader.read_u8().await?;
                let mut domain = String::new();
                reader.take(size as u64).read_to_string(&mut domain).await?;
                Address::Domain(domain)
            }
            4 => Address::IPv6(reader.read_u128().await?),
            _ => return Err(io::Error::new(ErrorKind::Other, "bad protocol")),
        };
        Ok(res)
    }

    pub async fn write_to<T: AsyncWrite + Unpin>(&self, writer: &mut T) -> io::Result<()> {
        match self {
            Address::IPv4(value) => {
                writer.write_u8(1).await?;
                writer.write_u32(*value).await?;
            }
            Address::Domain(domain) => {
                writer.write_u8(3).await?;
                writer.write_u8(domain.len() as u8).await?;
                writer.write_all(domain.as_bytes()).await?;
            }
            Address::IPv6(value) => {
                writer.write_u8(4).await?;
                writer.write_u128(*value).await?;
            }
        }
        Ok(())
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Address::IPv4(value) => write!(f, "{}", Ipv4Addr::from(*value)),
            Address::Domain(domain) => write!(f, "{}", domain),
            Address::IPv6(value) => write!(f, "{}", Ipv6Addr::from(*value)),
        }
    }
}

impl Default for Address {
    fn default() -> Self {
        Address::IPv4(0)
    }
}
