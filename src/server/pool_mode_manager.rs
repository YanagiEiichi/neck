use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::{io::AsyncBufReadExt, sync::Mutex};

use crate::{
    http::{HttpCommon, HttpRequest},
    neck::NeckStream,
};

use super::connection_manager::{ConnectingResult, ConnectionManager, PBFuture};

pub struct PoolModeManager {
    storage: Arc<Mutex<HashMap<SocketAddr, NeckStream>>>,
}

impl PoolModeManager {
    pub fn new() -> PoolModeManager {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn take(&self) -> Option<NeckStream> {
        let mut map = self.storage.lock().await;
        let key = *map.keys().into_iter().next()?;
        map.remove(&key)
    }
}

impl ConnectionManager for PoolModeManager {
    /// Get the current size of the pool.
    fn len(&self) -> PBFuture<usize> {
        Box::pin(async { self.storage.lock().await.len() })
    }

    /// Join the pool.
    fn join(&self, stream: NeckStream) -> PBFuture<()> {
        Box::pin(async {
            // The NeckStream will be moved later, so we need to clone necessary properties before moving.
            let addr = stream.peer_addr.clone();
            let am_reader = stream.reader.clone();

            // Store the NeckStream into the global pool (ownership has beed moved).
            self.storage.lock().await.insert(addr, stream);

            // To detect the EOF of a TcpStream, we must keep reading or peeking at it.
            // If this connection has been closed by peer and is still stored in the global pool,
            // we need remove the bad connection from global pool.
            // Otherwise, it could negatively impact other threads attempting to use the bad connection.
            let _ = am_reader.lock().await.fill_buf().await;

            self.storage.lock().await.remove(&addr);
        })
    }

    /// Attempt to acquire a NeckStream from the pool and establish the HTTP proxy connection.
    fn connect(&self, uri: String) -> PBFuture<ConnectingResult> {
        Box::pin(async move {
            // This is a retry loop, where certain operations can be retried, with a maximum of 5 retry attempts.
            for _ in 1..=5 {
                // Take a item from pool without retry.
                // If the pool is empty, retrying is pointless.
                let stream = match self.take().await {
                    Some(k) => k,
                    None => {
                        break;
                    }
                };

                // Send the PROXY request to upstream.
                // This operation can be retryed.
                if HttpRequest::new("CONNECT", &uri, "HTTP/1.1")
                    .write_to_stream(&stream)
                    .await
                    .is_err()
                {
                    continue;
                }

                // Read the first response from upstream.
                // This operation can be retryed.
                // let first_response = first_response.lock().await;
                let res = match stream.read_http_response().await {
                    Ok(res) => res,
                    Err(_) => {
                        continue;
                    }
                };

                // Got a non-200 status, this means proxy server cannot process this request, retrying is pointless.
                if res.get_status() != 200 {
                    let payload = res
                        .get_payload()
                        .as_ref()
                        .map_or_else(Vec::default, |v| v.to_vec());
                    let text = String::from_utf8(payload).unwrap_or_else(|e| e.to_string());
                    return ConnectingResult::ServiceUnavailable(text);
                }

                // Success, return the NeckStream object (transfer ownership).
                return ConnectingResult::Ok(stream);
            }

            // After too many retry attempts, a 502 status response is respond.
            ConnectingResult::BadGateway()
        })
    }
}
