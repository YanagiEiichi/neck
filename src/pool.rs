use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    http::{HttpCommonBasic, HttpRequestBasic, HttpResponse},
    neck::NeckStream,
};

pub(crate) struct Pool {
    storage: Arc<Mutex<HashMap<SocketAddr, NeckStream>>>,
}

pub enum ProxyResult {
    Ok(NeckStream),
    BadGateway(),
    ServiceUnavailable(String),
}

impl Pool {
    pub fn new() -> Pool {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the current size of the pool.
    pub async fn len(&self) -> usize {
        self.storage.lock().await.len()
    }

    /// Join the pool.
    pub async fn join(&self, stream: NeckStream) {
        // The NeckStream will be moved later, so we need to clone necessary properties before moving.
        let addr = stream.peer_addr.clone();
        let am_reader = stream.reader.clone();

        // Store the NeckStream into the global pool (ownership has beed moved).
        self.storage.lock().await.insert(addr, stream);

        // To detect the EOF of a TcpStream, we must keep reading or peeking at it.
        // If this connection has been closed by peer and is still stored in the global pool,
        // we need remove the bad connection from global pool.
        // Otherwise, it could negatively impact other threads attempting to use the bad connection.
        NeckStream::peek_one_byte(am_reader).await;
        self.storage.lock().await.remove(&addr);
    }

    async fn take(&self) -> Option<NeckStream> {
        let mut map = self.storage.lock().await;
        let key = *map.keys().into_iter().next()?;
        map.remove(&key)
    }

    /// Attempt to acquire a NeckStream from the pool and establish the HTTP proxy connection.
    pub async fn connect(&self, uri: &str) -> ProxyResult {
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
            match stream
                .write(HttpRequestBasic::new("CONNECT", uri, "HTTP/1.1", vec![]).to_string())
                .await
            {
                Ok(_) => (),
                Err(_) => {
                    continue;
                }
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
                return ProxyResult::ServiceUnavailable(res.get_payload().to_string());
            }

            // Success, return the NeckStream object (transfer ownership).
            return ProxyResult::Ok(stream);
        }

        // After too many retry attempts, a 502 status response is respond.
        ProxyResult::BadGateway()
    }
}
