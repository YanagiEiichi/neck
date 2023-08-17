use std::{collections::HashMap, net::SocketAddr, ops::Add, sync::Arc, time::Duration};

use tokio::{
    io::AsyncBufReadExt,
    sync::{Mutex, Notify},
    time::{timeout_at, Instant},
};

use crate::{
    http::{HttpCommon, HttpRequest, HttpResponse},
    neck::NeckStream,
};

use super::connection_manager::{ConnectingResult, ConnectionManager, PBFuture};

pub struct PoolModeManager {
    storage: Arc<Mutex<HashMap<SocketAddr, NeckStream>>>,
    notify: Arc<Notify>,
}

impl PoolModeManager {
    pub fn new() -> PoolModeManager {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
            notify: Arc::new(Notify::new()),
        }
    }

    async fn take(&self) -> Option<NeckStream> {
        // Declare a deadline.
        let deadline = Instant::now().add(Duration::from_secs(5));
        loop {
            // Try to take a NeckStream from pool.
            if let result @ Some(_) = {
                let mut map = self.storage.lock().await;
                map.keys()
                    .into_iter()
                    .next()
                    .map(|v| *v)
                    .map_or(None, |k| map.remove(&k))
            } {
                // If the NeckStream is take successfully, return it directly.
                return result;
            }
            // Otherwise, wait for a notification to retry it, and if the timeout occurs, return None.
            if timeout_at(deadline, self.notify.notified()).await.is_err() {
                return None;
            }
        }
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

            // Notify someone who is waiting to take a resource.
            self.notify.notify_one();

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
                    .add_header_kv("Host", &stream.peer_addr.to_string())
                    .write_to_stream(&stream)
                    .await
                    .is_err()
                {
                    continue;
                }

                // Read the first response from upstream.
                // This operation can be retryed.
                // let first_response = first_response.lock().await;
                let res = match HttpResponse::read_from(&stream).await {
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
