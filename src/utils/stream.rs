use std::{cell::UnsafeCell, future::Future, marker::PhantomPinned, net::SocketAddr, pin::Pin};

use tokio::{
    io::{self, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter},
    select,
    sync::Mutex,
};

use crate::{
    http::{HttpProtocol, HttpRequest, HttpResponse},
    socks5::Socks5Message,
    utils::NeckError,
};

use super::{NeckResult, SupportedStream};

pub struct NeckStream {
    raw: Mutex<Box<UnsafeCell<SupportedStream>>>,

    pub reader: Mutex<BufReader<Box<dyn AsyncRead + Send + Unpin>>>,
    pub writer: Mutex<Box<dyn AsyncWrite + Send + Unpin>>,

    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,

    // Pin this struct to prevent any properties from being taken out.
    // Because this struct contains unsafe pointers.
    // The `reader`, and `writer` refer to `raw`.
    _pinned: PhantomPinned,
}

impl<T: Into<SupportedStream>> From<T> for NeckStream {
    fn from(stream: T) -> Self {
        let ss = stream.into();
        let peer_addr = ss.get_tcp_stream_ref().peer_addr().unwrap();
        let local_addr = ss.get_tcp_stream_ref().local_addr().unwrap();

        // The UnsafeCell is currently on the stack, wrap it with a `Box` in order to move it to the heap.
        // This is an important operation since this pointer will be moved to the `Mutex` as a property of NeckStream.
        // The move operation will change its pointer address,
        // resulting in the unsafe dereference operation leading to a bad memory location.
        let buss = Box::new(UnsafeCell::new(ss));

        let (reader, writer) = SupportedStream::split(unsafe {
            // Pin this value to prevent moving the pointer out.
            // The UnsafeCell pointer must not be moved out.
            Pin::new_unchecked(&mut *buss.get())
        });

        Self {
            raw: Mutex::new(buss),
            writer: Mutex::new(writer),
            reader: Mutex::new(BufReader::with_capacity(10240, reader)),
            peer_addr,
            local_addr,
            _pinned: PhantomPinned,
        }
    }
}

impl NeckStream {
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
        let mut buf = Vec::new();
        if self
            .raw
            .lock()
            .await
            .get_mut()
            .get_tcp_stream_ref()
            .peek(&mut buf)
            .await?
            == 0
        {
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
