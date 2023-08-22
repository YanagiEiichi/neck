mod direct;
mod pool;

use std::{future::Future, pin::Pin};

use crate::neck::NeckStream;

pub use pool::*;
pub use direct::*;

pub enum ConnectingResult {
    Ok(NeckStream),
    BadGateway(),
    ServiceUnavailable(String),
}

pub type PBFuture<'a, O> = Pin<Box<dyn Future<Output = O> + Send + 'a>>;

pub trait ConnectionManager: Send + Sync {
    /// Get the number of current avaliable connections.
    fn len(&self) -> PBFuture<usize>;

    /// Join the manager.
    fn join(&self, stream: NeckStream) -> PBFuture<()>;

    /// Attempt to acquire a NeckStream from the manager.
    fn connect(&self, uri: String) -> PBFuture<ConnectingResult>;
}
