use std::{collections::HashMap, net::SocketAddr, ops::Add, sync::Arc, time::Duration};

use tokio::{
    sync::{Mutex, Notify},
    time::{timeout_at, Instant},
};

use crate::{
    http::{HttpCommon, HttpRequest, HttpResponse},
    server::session_manager::Session,
    utils::NeckStream,
};

use super::{ConnectingResult, ConnectionManager, PBF};

pub struct PoolModeManager {
    size: usize,
    storage: Arc<Mutex<HashMap<SocketAddr, Arc<NeckStream>>>>,
    conn_joined: Arc<Notify>,
}

impl PoolModeManager {
    pub fn new(size: usize) -> PoolModeManager {
        Self {
            size,
            storage: Arc::new(Mutex::new(HashMap::new())),
            conn_joined: Arc::new(Notify::new()),
        }
    }

    async fn take(&self) -> Option<Arc<NeckStream>> {
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
            if let Err(_) = timeout_at(deadline, self.conn_joined.notified()).await {
                return None;
            }
        }
    }

    async fn take_and_send_connect(&self, uri: &str) -> Option<Arc<NeckStream>> {
        // This is a retry loop, where certain operations can be retried, with a maximum of 5 retry attempts.
        for _ in 1..=5 {
            // Take a item from pool without retry.
            // If the pool is empty, retrying is pointless.
            let stream = match self.take().await {
                Some(k) => k,
                None => break,
            };

            // Send CONNECT reqeust.
            if let Err(_) = HttpRequest::new("CONNECT", &uri, "HTTP/1.1")
                .add_header_kv("Host", &stream.peer_addr.to_string())
                .write_to_stream(&stream)
                .await
            {
                continue;
            };

            return Some(stream);
        }
        None
    }

    /// Try to insert a `stream` to the pool and send a notification if fuccessful.
    /// If the pool is already full, the `stream` will be dropped.
    async fn try_insert(&self, stream: Arc<NeckStream>) -> bool {
        let mut s = self.storage.lock().await;

        // Check the pool size, if it is already full, return false directly.
        // NOTE: The ownership of `stream` will not be returned here, it will be dropped.
        if s.len() >= self.size {
            return false;
        }

        // Insert the `stream` into the pool (ownership has been moved).
        let addr = stream.peer_addr;
        s.insert(addr, stream);

        // When a stream is inserted to the pool, notify a waiting routine to attempt to retrieval.
        self.conn_joined.notify_one();

        true
    }

    /// If a connection is closed by peer, it will be remove fastly, to prevent it from being used in other routines.
    async fn wait_close_or_use(&self, addr: SocketAddr) {
        // Get the reader pointer.
        let stream = match self.storage.lock().await.get(&addr) {
            Some(s) => s.clone(),
            None => return,
        };

        // If the `reader` receives anything, remove it from the pool and stop the health check loop.
        // There are two cases for this:
        // 1. The `stream`, which is still in the pool, but closed by peer.
        // 2. The `stream` has been taken out by another routine, and has been used.
        // If this connection has closed by peer.
        let _ = stream.quick_check_eof().await;

        // It probably has already been removed by another routine, but do not care about that.
        self.storage.lock().await.remove(&addr);
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
            let addr = stream.peer_addr;

            // Try to join the pool, if it is failed not, return this function.
            if !self.try_insert(Arc::new(stream)).await {
                return;
            }

            // Otherwise, the stream has joined the pool.

            self.wait_close_or_use(addr).await;
        })
    }

    /// Attempt to acquire a NeckStream from the pool and establish the HTTP proxy connection.
    fn connect<'a>(&'a self, session: &'a Session) -> PBF<'a, ConnectingResult> {
        let ss = Arc::new(session);
        Box::pin(async move {
            let stream = match self.take_and_send_connect(&ss.host).await {
                Some(it) => it,
                None => return ConnectingResult::BadGateway(),
            };

            session.set_it_connecting();

            // Receive an HTTP response.
            let res = match HttpResponse::read_from(&stream).await {
                Ok(it) => it,
                Err(e) => return ConnectingResult::ServiceUnavailable(e.to_string()),
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

            session.set_it_established();

            // Success, return the NeckStream object (transfer ownership).
            return ConnectingResult::Ok(stream);
        })
    }
}
