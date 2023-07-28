use std::{error::Error, fmt::Display};

use tokio::{
    io::{self, AsyncRead, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};

use crate::http::HttpProtocol;

pub async fn weld_for_rw<R, W>(a: &mut TcpStream, br: &mut R, bw: &mut W)
where
    R: AsyncRead,
    R: Unpin,
    W: AsyncWrite,
    W: Unpin,
{
    let (mut ar, mut aw) = a.split();
    let t1 = io::copy(&mut ar, bw);
    let t2 = io::copy(br, &mut aw);
    tokio::select! {
      _ = t1 => {}
      _ = t2 => {}
    };
}

pub async fn weld(a: &mut TcpStream, b: &mut TcpStream) {
    let (mut br, mut bw) = b.split();
    weld_for_rw(a, &mut br, &mut bw).await;
}

pub async fn respond_without_body(
    stream: &mut TcpStream,
    status: u16,
    text: &str,
    version: &str,
) -> Result<usize, io::Error> {
    let res = HttpProtocol::new(
        (
            String::from(version),
            status.to_string(),
            String::from(text),
        ),
        Vec::new(),
    );
    stream.write(res.to_string().as_bytes()).await
}

pub async fn respond(
    stream: &mut TcpStream,
    status: u16,
    text: &str,
    version: &str,
    payload: &str,
) -> Result<usize, std::io::Error> {
    let mut headers = Vec::new();
    headers.push(String::from("Content-Type: text/plain"));
    headers.push(format!("Content-Length: {}", payload.as_bytes().len()));
    let res = HttpProtocol::new(
        (
            String::from(version),
            status.to_string(),
            String::from(text),
        ),
        headers,
    )
    .to_string()
        + payload;
    stream.write(res.as_bytes()).await
}

#[derive(Debug)]
pub struct NeckError {
    message: String,
}

impl NeckError {
    pub fn new(message: String) -> NeckError {
        NeckError { message }
    }
}

impl Display for NeckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for NeckError {}
