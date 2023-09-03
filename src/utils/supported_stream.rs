use std::pin::Pin;

use tokio::{
    io::{split, AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_native_tls::TlsStream;

pub enum SupportedStream {
    Tls(TlsStream<TcpStream>),
    Tcp(TcpStream),
}

impl SupportedStream {
    pub fn split<'a>(
        self: Pin<&'a mut Self>,
    ) -> (
        Box<dyn AsyncRead + Unpin + Send + 'a>,
        Box<dyn AsyncWrite + Unpin + Send + 'a>,
    ) {
        match self.get_mut() {
            SupportedStream::Tls(s) => {
                let (r, w) = split(s);
                (Box::new(r), Box::new(w))
            }
            SupportedStream::Tcp(s) => {
                let (r, w) = split(s);
                (Box::new(r), Box::new(w))
            }
        }
    }

    pub fn get_tcp_stream_ref(&self) -> &TcpStream {
        match self {
            SupportedStream::Tls(s) => s.get_ref().get_ref().get_ref(),
            SupportedStream::Tcp(s) => s,
        }
    }
}

impl Into<SupportedStream> for TcpStream {
    fn into(self) -> SupportedStream {
        SupportedStream::Tcp(self)
    }
}

impl Into<SupportedStream> for TlsStream<TcpStream> {
    fn into(self) -> SupportedStream {
        SupportedStream::Tls(self)
    }
}
