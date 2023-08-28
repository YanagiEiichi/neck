use std::{future::Future, net::SocketAddr, os::fd::FromRawFd, sync::Arc};

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(windows)]
use std::os::windows::io::AsRawSocket;

use tokio::{
    io::{
        self, split, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
    },
    net::TcpStream,
    select,
    sync::Mutex,
};

use crate::{
    http::{HttpProtocol, HttpRequest, HttpResponse},
    socks5::Socks5Message,
    utils::NeckError,
};

use super::NeckResult;

pub struct NeckStream {
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub writer: Mutex<Box<dyn AsyncWrite + Send + Unpin>>,
    pub reader: Arc<Mutex<BufReader<Box<dyn AsyncRead + Send + Unpin>>>>,
    pub fd: i32,
}

impl NeckStream {
    pub fn new<T>(peer_addr: SocketAddr, local_addr: SocketAddr, stream: T, fd: i32) -> Self
    where
        T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let (r, w) = split(stream);
        Self {
            peer_addr,
            local_addr,
            writer: Mutex::new(Box::new(w)),
            reader: Arc::new(Mutex::new(BufReader::with_capacity(10240, Box::new(r)))),
            fd,
        }
    }

    /// Weld with another NeckStream (Start a bidirectional stream copy).
    /// After welding, do not use these streams elsewhere because both streams will be fully consumed.
    pub async fn weld(&self, upstream: &Self) {
        // Split and lock all half streams.
        let (mut ar, mut aw, mut br, mut bw) = tokio::join!(
            self.reader.lock(),
            self.writer.lock(),
            upstream.reader.lock(),
            upstream.writer.lock()
        );

        // Weld them together.
        let t1 = io::copy(&mut *ar, &mut *bw);
        let t2 = io::copy(&mut *br, &mut *aw);

        // Use `select!` rather than `join!` here. Because the `join!` waits for both copying tasks to complete,
        // but an HTTP client may still be in the half-closing, which will hang the connection and not release it.
        // The `select!` indicates that either or the task completes. Therefore, both stream will be Drop and released.
        select! {
          _ = t1 => (),
          _ = t2 => ()
        }
    }

    /// Shutdown the connection immediately.
    pub async fn shutdown(&self) -> io::Result<()> {
        self.writer.lock().await.shutdown().await
    }

    async fn wait_until_close<T>(&self) -> NeckResult<T> {
        let mut reader = self.reader.lock().await;

        // Fast check.
        let buf = reader.fill_buf().await?;
        if buf.is_empty() {
            return NeckError::wrap("Closed by peer");
        }

        // Try to peek raw TCP socket.
        #[cfg(unix)]
        let copy = unsafe { std::net::TcpStream::from_raw_fd(self.fd) };
        #[cfg(windows)]
        let copy = unsafe { std::net::TcpStream::from_raw_socket(self.fd) };
        let mut buf = Vec::new();
        if TcpStream::from_std(copy)?.peek(&mut buf).await? == 0 {
            return NeckError::wrap("Closed by peer");
        }

        return NeckError::wrap("Buffer overflow");
    }

    pub async fn wait_toggle<T>(&self, task: impl Future<Output = T>) -> NeckResult<T> {
        select! {
          v = task => Ok(v),
          v = self.wait_until_close() => v
        }
    }
}

impl From<TcpStream> for NeckStream {
    fn from(stream: TcpStream) -> Self {
        let peer_addr = stream.peer_addr().unwrap();
        let local_addr = stream.local_addr().unwrap();

        #[cfg(unix)]
        let fd = stream.as_raw_fd();
        #[cfg(windows)]
        let fd = stream.as_raw_socket();

        Self::new(peer_addr, local_addr, stream, fd)
    }
}

impl HttpProtocol {
    pub async fn write_to_stream(&self, stream: &NeckStream) -> io::Result<()> {
        let mut writer = stream.writer.lock().await;
        let mut w = BufWriter::with_capacity(1480, &mut *writer);
        self.write_to(&mut w).await?;
        w.flush().await?;
        Ok(())
    }
}

impl HttpRequest {
    /// Read an HTTP request, and wait for an HTTP request to be received completely.
    pub async fn read_from(stream: &NeckStream) -> io::Result<HttpRequest> {
        let mut reader = stream.reader.lock().await;
        HttpProtocol::read_from(&mut reader).await.map(|v| v.into())
    }

    /// Read an HTTP request (wait for an HTTP request to be received completely).
    /// NOTE: The payload will not be readed.
    pub async fn read_header_from(stream: &NeckStream) -> io::Result<HttpRequest> {
        let mut reader = stream.reader.lock().await;
        HttpProtocol::read_header_from(&mut reader)
            .await
            .map(|v| v.into())
    }
}

impl HttpResponse {
    /// Read an HTTP response (wait for an HTTP response to be received completely).
    pub async fn read_from(stream: &NeckStream) -> io::Result<HttpResponse> {
        let mut reader = stream.reader.lock().await;
        HttpProtocol::read_from(&mut reader).await.map(|v| v.into())
    }
}

impl Socks5Message {
    pub async fn write_to_stream(&self, stream: &NeckStream) -> io::Result<()> {
        let mut writer = stream.writer.lock().await;
        let mut w = BufWriter::with_capacity(1480, &mut *writer);
        self.write_to(&mut w).await?;
        w.flush().await?;
        Ok(())
    }
}
