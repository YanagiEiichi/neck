use std::{collections::HashMap, net::SocketAddr, ops::Add, sync::Arc, time::Duration};

use rand::Rng;
use tokio::{
    io::{self, AsyncBufReadExt},
    sync::{Mutex, Notify},
    time::{timeout, timeout_at, Instant},
};

use crate::{
    http::{HttpCommon, HttpRequest, HttpResponse},
    neck::NeckStream,
    utils::{NeckError, NeckResult},
};

use super::{ConnectingResult, ConnectionManager, PBF};

pub struct PoolModeManager {
    size: usize,
    storage: Arc<Mutex<HashMap<SocketAddr, NeckStream>>>,
    // TODO: rename
    notify: Arc<Notify>,
}

impl PoolModeManager {
    pub fn new(size: usize) -> PoolModeManager {
        Self {
            size,
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

    /// Try to insert a `stream` to the pool and send a notification if fuccessful.
    /// If the pool is already full, the `stream` will be dropped.
    async fn try_insert(&self, stream: NeckStream) -> bool {
        let mut s = self.storage.lock().await;

        // Check the pool size, if it is already full, return false directly.
        // NOTE: The ownership of `stream` will not be returned here, it will be dropped.
        if s.len() >= self.size {
            return false;
        }

        // Insert the `stream` into the pool (ownership has been moved).
        let addr = stream.peer_addr.clone();
        s.insert(addr, stream);

        // When a stream is inserted to the pool, notify a waiting routine to attempt to retrieval.
        self.notify.notify_one();

        true
    }

    /// Keep the `stream` alive while it lives in the pool.
    ///
    /// Firstly, we need to keep the `stream` being reading, because we must know a FIN packet received.
    /// When a FIN packet is received, this indicates that the `stream` has closed,
    /// and we should remove this `stream` from the pool to prevent it from being used in other routines.
    ///
    /// However, merely reading the `stream` is insufficient in this context.
    /// Because a FIN packet might be lose in the network, the TCP connection could be "dead".
    /// Therefore, we should initiate a health check loop to identify this "dead" scenario.
    ///
    async fn initiate_health_check_loop(&self, addr: SocketAddr) {
        // Get the reader pointer.
        let reader = match self.storage.lock().await.get(&addr) {
            Some(stream) => stream.reader.clone(),
            None => return,
        };

        // Initiate the health check loop.
        loop {
            // If the `reader` receives anything, remove it from the pool and stop the health check loop.
            // There are two cases for this:
            // 1. The `stream`, which is still in the pool, receives an EOF from peer.
            // 2. The `stream` has been taken out by another routine, and has been used.
            let secs = rand::thread_rng().gen_range(60..120);
            if let Ok(_) = timeout(
                // Set a maximum waiting duration.
                Duration::from_secs(secs),
                // The `fill_buf` method will wait if its buffer is empty.
                reader.lock().await.fill_buf(),
            )
            .await
            {
                // It probably has already been removed by another routine, but we do not care about that.
                self.storage.lock().await.remove(&addr);
                break;
            };

            // Otherwise, nothing to receive, just timing out.

            // Try to take out the stream from pool.
            // If it has already been removed by another routine, stop the health check loop immediately.
            let stream = match self.storage.lock().await.remove(&addr) {
                Some(v) => v,
                None => break,
            };

            // Execute an HTTP health check.
            // If it is failed, stop the health check loop immediately.
            if let Err(_) = send_ping_and_assert_pong(&stream).await {
                // In this case, the `stream` will not be inserted back into the pool.
                // While this stack is exited, the `stream` will be dropped.
                break;
            }

            // Insert the `stream` back into the pool.
            // If the operation fails (for example, if the pool is full capacity), the `stream` will be dropped.
            if !self.try_insert(stream).await {
                break;
            }
        }
    }
}

impl ConnectionManager for PoolModeManager {
    /// Get the current size of the pool.
    fn len(&self) -> PBF<usize> {
        Box::pin(async { self.storage.lock().await.len() })
    }

    /// Join the pool.
    fn join(&self, stream: NeckStream) -> PBF<()> {
        Box::pin(async {
            // The NeckStream will be moved later, so we need to clone necessary properties before moving.
            let addr = stream.peer_addr.clone();

            // Try to join the pool, if it is failed not, return this function.
            if !self.try_insert(stream).await {
                return;
            }

            // Otherwise, the stream has joined the pool.
            self.initiate_health_check_loop(addr).await;
        })
    }

    /// Attempt to acquire a NeckStream from the pool and establish the HTTP proxy connection.
    fn connect(&self, uri: String) -> PBF<ConnectingResult> {
        Box::pin(async move {
            // This is a retry loop, where certain operations can be retried, with a maximum of 5 retry attempts.
            for _ in 1..=5 {
                // Take a item from pool without retry.
                // If the pool is empty, retrying is pointless.
                let stream = match self.take().await {
                    Some(k) => k,
                    None => break,
                };

                // Send a CONNECT request and receive an HTTP response.
                let res = match send_connect_and_receive_response(&uri, &stream).await {
                    Ok(r) => r,
                    // This operation can be retried.
                    Err(_) => continue,
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

/// Send a CONNECT request and receive an HTTP response.
async fn send_connect_and_receive_response(
    uri: &String,
    stream: &NeckStream,
) -> io::Result<HttpResponse> {
    // Send CONNECT reqeust.
    HttpRequest::new("CONNECT", uri, "HTTP/1.1")
        .add_header_kv("Host", &stream.peer_addr.to_string())
        .write_to_stream(stream)
        .await?;

    // Receive an HTTP response.
    Ok(HttpResponse::read_from(stream).await?)
}

/// Send a PING request and assert a PONG response.
async fn send_ping_and_assert_pong(stream: &NeckStream) -> NeckResult<()> {
    // Send PING request.
    HttpRequest::new("PING", "*", "HTTP/1.1")
        .write_to_stream(&stream)
        .await?;

    // Receive response and assert status code 204.
    let res = HttpResponse::read_from(stream).await?;
    if res.get_status() != 204 {
        NeckError::wrap("Got non-204 status when PING")?
    }

    Ok(())
}
