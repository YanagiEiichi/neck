mod direct;
mod pool;

use crate::utils::{NeckStream, PBF};

pub use direct::*;
pub use pool::*;

use super::session_manager::Session;

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
    fn connect<'a>(&'a self, session: &'a Session) -> PBF<'a, ConnectingResult>;
}
