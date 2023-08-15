use std::{error::Error, net::SocketAddr, sync::Arc};

use tokio::{
    io::{self, split, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    net::TcpStream,
    select,
    sync::Mutex,
};

use crate::http::{HttpProtocol, HttpRequest, HttpResponse};

pub struct NeckStream {
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub writer: Mutex<Box<dyn AsyncWrite + Send + Unpin>>,
    pub reader: Arc<Mutex<BufReader<Box<dyn AsyncRead + Send + Unpin>>>>,
}

// impl Drop for NeckStream {
//     fn drop(&mut self) {
//         println!("Drop {}", self.peer_addr);
//     }
// }

impl NeckStream {
    pub fn new<T>(peer_addr: SocketAddr, local_addr: SocketAddr, stream: T) -> Self
    where
        T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let (r, w) = split(stream);
        Self {
            peer_addr,
            local_addr,
            writer: Mutex::new(Box::new(w)),
            reader: Arc::new(Mutex::new(BufReader::new(Box::new(r)))),
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
    pub async fn shutdown(&self) -> Result<(), impl Error> {
        self.writer.lock().await.shutdown().await
    }
}

impl From<TcpStream> for NeckStream {
    fn from(stream: TcpStream) -> Self {
        let peer_addr = stream.peer_addr().unwrap();
        let local_addr = stream.local_addr().unwrap();
        Self::new(peer_addr, local_addr, stream)
    }
}

impl HttpProtocol {
    pub async fn write_to_stream(&self, stream: &NeckStream) -> Result<(), Box<dyn Error>> {
        let mut writer = stream.writer.lock().await;
        self.write_to(&mut *writer).await
    }
}

impl HttpRequest {
    /// Read an HTTP request, and wait for an HTTP request to be received completely.
    pub async fn read_from(stream: &NeckStream) -> Result<HttpRequest, Box<dyn Error>> {
        let mut reader = stream.reader.lock().await;
        HttpProtocol::read_from(&mut reader).await.map(|v| v.into())
    }

    /// Read an HTTP request (wait for an HTTP request to be received completely).
    /// NOTE: The payload will not be readed.
    pub async fn read_header_from(stream: &NeckStream) -> Result<HttpRequest, Box<dyn Error>> {
        let mut reader = stream.reader.lock().await;
        HttpProtocol::read_header_from(&mut reader)
            .await
            .map(|v| v.into())
    }
}

impl HttpResponse {
    /// Read an HTTP response (wait for an HTTP response to be received completely).
    pub async fn read_from(stream: &NeckStream) -> Result<HttpResponse, Box<dyn Error>> {
        let mut reader = stream.reader.lock().await;
        HttpProtocol::read_from(&mut reader).await.map(|v| v.into())
    }
}
