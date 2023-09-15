use std::{
    future::Future, marker::PhantomPinned, net::SocketAddr, pin::Pin, ptr::addr_of_mut,
    time::Duration,
};

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
    raw: Box<SupportedStream>,

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
        // This is an important operation since this pointer will be moved to the `Mutex` as a property of NeckStream.
        // The move operation will change its pointer address,
        // resulting in the unsafe dereference operation leading to a bad memory location.
        let mut buss = Box::new(stream.into());

        let (reader, writer) = SupportedStream::split(unsafe {
            // Pin this value to prevent moving the pointer out.
            Pin::new_unchecked(&mut *addr_of_mut!(*buss.as_mut()))
        });

        let peer_addr = buss.get_tcp_stream_ref().peer_addr().unwrap();
        let local_addr = buss.get_tcp_stream_ref().local_addr().unwrap();

        Self {
            raw: buss,
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

    /// Perform a quick check using the standard `fill_buf` method of `BufReader`.
    /// This method will wait until any bytes are received from system buffer, unless it receives an EOF.
    /// Therefore, if this buffer is empty, it indicates that this connection has been closed by peer.
    pub async fn quick_check_eof(&self) -> NeckResult<()> {
        let mut reader = self.reader.lock().await;
        let buf = reader.fill_buf().await?;
        if buf.is_empty() {
            return NeckError::wrap("Closed by peer");
        }
        Ok(())
    }

    /// Get the raw `TcpStream` and peek it.
    pub async fn peek_raw_tck_stream(&self) -> Result<usize, io::Error> {
        let mut buf = [0; 1];
        self.raw.get_tcp_stream_ref().peek(&mut buf).await
    }

    /// Wait until this connection closed by peer.
    pub async fn wait_until_close<T>(&self) -> NeckResult<T> {
        self.quick_check_eof().await?;

        // Regarding detecting a FIN, this is a complex problem.
        // Here, I am simply polling using `peek` mehtod on the system buffer.
        // If the system buffer is empty, it indicates that this connection has been closed by peer.
        // If the system buffer is truly empty but not FIN-ed, the `peek` will wait until any bytes are received,
        // so, if zero size buffer are peeked, it guarantee that the stream has reached EOF, i.e. TCP received a FIN.
        //
        // In fact, there are more complex cases:
        //
        // CASE 1: The buffer is empty, nothing to receive.
        //         The routine will be waiting at `peek` until any bytes are received.
        // CASE 2: The buffer is empty, but a FIN is received.
        //         The `while` loop will break, return an `Err`.
        // CASE 3: The buffer is not empty, data has not been read into the application layer.
        //         Polling `peek` until all system buffer read by application layer, i.e., read by `BufReader`.
        //         This is an ugliy way, but I have not better solution to handler this case.
        // CASE 4: The buffer is not empty, but a FIN received.
        //         Emmm, I can only wish the `BufReader` reads all bytes fastly.
        //         Because receive the FIN and replay a ACT is the system behavior,
        //         this thing could not be notified to application layer.
        //         However, polling the `TCP_INFO` using the system API `getsockopt` can retrieve the connection state.
        //         A CLOSE_WAIT connection indicates that a FIN has received,
        //         but this solution is too messy as the system APIs differs between operating systems,
        //         I do not want write complex shit here.
        // CASE 5: The buffer is full, TCP window size is fully utilized.
        //         This is hell case. The client would not be able to send a FIN packet since the buffer if full,
        //         server side has no way to know client sent a FIN.
        //
        while self.peek_raw_tck_stream().await? > 0 {
            tokio::time::sleep(Duration::from_secs(1)).await
        }

        return NeckError::wrap("Closed by peer");
    }

    /// Wait for a `Future` while this connection is being ESTABLISHED.
    /// In other words, if this connection closed by peer, abandon waiting for the `Future`.
    pub async fn wait_together<T>(&self, task: impl Future<Output = T>) -> NeckResult<T> {
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
