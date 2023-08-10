use std::{
    io::Error,
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::AsyncWrite;

pub struct StringWritter {
    buffer: Vec<u8>,
}

#[allow(dead_code)]
impl StringWritter {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }
}

impl ToString for StringWritter {
    fn to_string(&self) -> String {
        String::from_utf8(self.buffer.clone()).unwrap()
    }
}

impl AsyncWrite for StringWritter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        self.buffer.extend(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}
