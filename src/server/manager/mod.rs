mod direct;
mod pool;

use crate::{neck::NeckStream, utils::PBF};

pub use pool::*;
pub use direct::*;

pub enum ConnectingResult {
    Ok(NeckStream),
    BadGateway(),
    ServiceUnavailable(String),
}


pub trait ConnectionManager: Send + Sync {
    /// Get the number of current avaliable connections.
    fn len(&self) -> PBF<usize>;

    /// Join the manager.
    fn join(&self, stream: NeckStream) -> PBF<()>;

    /// Attempt to acquire a NeckStream from the manager.
    fn connect(&self, uri: String) -> PBF<ConnectingResult>;
}
