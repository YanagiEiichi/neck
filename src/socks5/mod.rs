use std::{
    io::ErrorKind,
    net::{Ipv4Addr, Ipv6Addr},
};

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
