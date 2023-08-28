use crate::{
    server::session_manager::Session,
    utils::{connect, NeckStream},
};

use super::{ConnectingResult, ConnectionManager, PBF};

pub struct DirectModeManager {}

impl ConnectionManager for DirectModeManager {
    fn len(&self) -> PBF<usize> {
        // Always return zero.
        Box::pin(async { 0 })
    }

    fn join(&self, _stream: NeckStream) -> PBF<()> {
        // There is nothing to do.
        // Joined connection will lose all references and will be recycled later.
        Box::pin(async move {})
    }

    fn connect<'a>(&'a self, session: &'a Session) -> PBF<'a, ConnectingResult> {
        Box::pin(async move {
            // Pass through the tokio TcpStream::connect.
            match connect(&session.host).await {
                Ok(stream) => ConnectingResult::Ok(stream.into()),
                Err(e) => ConnectingResult::ServiceUnavailable(e.to_string()),
            }
        })
    }
}
