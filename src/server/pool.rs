use std::{collections::HashMap, future::Future, net::SocketAddr, pin::Pin, sync::Arc};

use tokio::{io::AsyncBufReadExt, net::TcpStream, sync::Mutex};

use crate::{http::HttpCommon, neck::NeckStream};

pub struct Pool {
    storage: Arc<Mutex<HashMap<SocketAddr, NeckStream>>>,
}

pub enum ProxyResult {
    Ok(NeckStream),
    BadGateway(),
    ServiceUnavailable(String),
}

type PBFuture<'a, O> = Pin<Box<dyn Future<Output = O> + Send + 'a>>;

pub trait Hub: Send + Sync {
    /// Get the current size of the pool.
    fn len(&self) -> PBFuture<usize>;

    /// Join the pool.
    fn join(&self, stream: NeckStream) -> PBFuture<()>;

    /// Attempt to acquire a NeckStream from the pool and establish the HTTP proxy connection.
    fn connect(&self, uri: String) -> PBFuture<ProxyResult>;
}

impl Pool {
    pub fn new() -> Pool {
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

impl Hub for Pool {
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
    fn connect(&self, uri: String) -> PBFuture<ProxyResult> {
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
                match stream.request("CONNECT", &uri, "HTTP/1.1", vec![]).await {
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
                    let text = String::from_utf8(res.get_payload().to_vec())
                        .unwrap_or_else(|e| e.to_string());
                    return ProxyResult::ServiceUnavailable(text);
                }

                // Success, return the NeckStream object (transfer ownership).
                return ProxyResult::Ok(stream);
            }

            // After too many retry attempts, a 502 status response is respond.
            ProxyResult::BadGateway()
        })
    }
}

pub struct MockPool {}

impl Hub for MockPool {
    fn len(&self) -> PBFuture<usize> {
        Box::pin(async { 0 })
    }

    fn join(&self, _stream: crate::neck::NeckStream) -> PBFuture<()> {
        Box::pin(async {})
    }

    fn connect(&self, uri: String) -> PBFuture<crate::server::ProxyResult> {
        Box::pin(async move {
            match TcpStream::connect(&uri).await {
                Ok(stream) => ProxyResult::Ok(stream.into()),
                Err(e) => ProxyResult::ServiceUnavailable(e.to_string()),
            }
        })
    }
}
