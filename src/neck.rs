use std::{error::Error, net::SocketAddr, sync::Arc};

use tokio::{
    io::{self, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    select,
    sync::Mutex,
};

use crate::http::{FirstLine, Headers, HttpProtocol, HttpRequest, HttpResponse};

pub struct NeckStream {
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub writer: Mutex<OwnedWriteHalf>,
    pub reader: Arc<Mutex<BufReader<OwnedReadHalf>>>,
}

// impl Drop for NeckStream {
//     fn drop(&mut self) {
//         println!("Drop {}", self.peer_addr);
//     }
// }

impl NeckStream {
    pub fn new(stream: TcpStream) -> NeckStream {
        let peer_addr = stream.peer_addr().unwrap();
        let local_addr = stream.local_addr().unwrap();
        let (orh, owh) = stream.into_split();
        let reader = Arc::new(Mutex::new(BufReader::new(orh)));
        let writer = Mutex::new(owh);
        NeckStream {
            peer_addr,
            local_addr,
            writer,
            reader,
        }
    }

    /// Read an HTTP request (wait for an HTTP request to be received completely).
    pub async fn read_http_request(&self) -> Result<HttpRequest, Box<dyn Error>> {
        let mut reader = self.reader.lock().await;
        HttpRequest::read_from(&mut reader).await
    }

    /// Read an HTTP response (wait for an HTTP response to be received completely).
    pub async fn read_http_response(&self) -> Result<HttpResponse, Box<dyn Error>> {
        let mut reader = self.reader.lock().await;
        HttpResponse::read_from(&mut reader).await
    }

    /// Send an HTTP response.
    pub async fn respond(
        &self,
        status: u16,
        text: &str,
        version: &str,
        payload: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut headers = Vec::new();
        headers.push(String::from("Content-Type: text/plain"));
        headers.push(format!("Content-Length: {}", payload.as_bytes().len()));
        HttpProtocol::new(
            FirstLine::new(version, &status.to_string(), text),
            headers,
            payload.as_bytes().to_vec(),
        )
        .write_to(&mut *self.writer.lock().await)
        .await
    }

    /// Send an HTTP request.
    pub async fn request(
        &self,
        method: &str,
        uri: &str,
        version: &str,
        headers: impl Into<Headers>,
    ) -> Result<(), Box<dyn Error>> {
        HttpProtocol::new(FirstLine::new(method, uri, version), headers, Vec::new())
            .write_to(&mut *self.writer.lock().await)
            .await
    }

    pub async fn peek_one_byte(am_reader: Arc<Mutex<BufReader<OwnedReadHalf>>>) -> usize {
        let mut buf_reader = am_reader.lock().await;
        let raw_reader = buf_reader.get_mut();
        let mut buf = [0u8; 1];
        raw_reader.peek(&mut buf).await.unwrap()
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
