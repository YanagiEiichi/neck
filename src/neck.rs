use std::{error::Error, net::SocketAddr};

use tokio::{
    io::{self, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    select,
    sync::Mutex,
};

use crate::http::{HttpProtocol, HttpRequestBasic, HttpResponseBasic};

pub struct NeckStream {
    pa: SocketAddr,
    la: SocketAddr,
    w: Mutex<OwnedWriteHalf>,
    r: Mutex<BufReader<OwnedReadHalf>>,
}

impl NeckStream {
    pub fn new(stream: TcpStream) -> NeckStream {
        let pa: SocketAddr = stream.peer_addr().unwrap();
        let la: SocketAddr = stream.local_addr().unwrap();
        let (orh, owh) = stream.into_split();
        let r = Mutex::new(BufReader::new(orh));
        let w = Mutex::new(owh);
        NeckStream { pa, la, w, r }
    }

    /// Borrow local addr.
    pub fn local_addr(&self) -> &SocketAddr {
        &self.la
    }

    /// Borrow peer addr.
    pub fn peer_addr(&self) -> &SocketAddr {
        &self.pa
    }

    /// Read an HTTP request (wait for an HTTP request to be received completely).
    pub async fn read_http_request(&self) -> Result<HttpRequestBasic, Box<dyn Error>> {
        let mut reader = self.r.lock().await;
        HttpRequestBasic::read_from(&mut reader).await
    }

    /// Read an HTTP response (wait for an HTTP response to be received completely).
    pub async fn read_http_response(&self) -> Result<HttpResponseBasic, Box<dyn Error>> {
        let mut reader = self.r.lock().await;
        HttpResponseBasic::read_from(&mut reader).await
    }

    /// Write a string to writter.
    pub async fn write(&self, data: String) -> Result<usize, std::io::Error> {
        let mut writer = self.w.lock().await;
        writer.write(data.as_bytes()).await
    }

    /// Send an HTTP response.
    pub async fn respond(
        &self,
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
        let mut writer = self.w.lock().await;
        writer.write(res.as_bytes()).await
    }

    /// Borrow the write and read half streams.
    pub fn split(&self) -> (&Mutex<BufReader<OwnedReadHalf>>, &Mutex<OwnedWriteHalf>) {
        (&self.r, &self.w)
    }

    /// Weld with another NeckStream (Start a bidirectional stream copy).
    /// After welding, do not use these streams elsewhere because both streams will be fully consumed.
    pub async fn weld(&self, upstream: &Self) {
        // Split and lock all half streams.
        let (mar, maw) = self.split();
        let (mbr, mbw) = upstream.split();
        let (mut ar, mut aw, mut br, mut bw) =
            tokio::join!(mar.lock(), maw.lock(), mbr.lock(), mbw.lock());

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
    pub async fn shutdown(&self) -> Result<(), io::Error> {
        self.w.lock().await.shutdown().await
    }
}
