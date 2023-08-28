use std::{future::Future, pin::Pin, time::Duration};

mod error;
mod stream;

pub use error::*;
use socket2::{Socket, TcpKeepalive};
pub use stream::*;
use tokio::net::TcpStream;

/// PBF = Pin Box Future
pub type PBF<'a, O> = Pin<Box<dyn Future<Output = O> + Send + 'a>>;

pub type NeckResult<T> = Result<T, BoxedError>;

impl NeckError {
    pub fn wrap<T>(message: impl ToString) -> NeckResult<T> {
        Err(Box::new(NeckError::new(message)))
    }
}

pub fn enable_keepalive(stream: TcpStream) -> TcpStream {
    let socket = Socket::from(stream.into_std().unwrap());
    let keepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(4))
        .with_interval(Duration::from_secs(1))
        .with_retries(4);
    socket.set_tcp_keepalive(&keepalive).unwrap();
    TcpStream::from_std(socket.into()).unwrap()
}
