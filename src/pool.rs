use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    http::{HttpCommonBasic, HttpRequestBasic, HttpResponse, HttpResponseBasic},
    neck::NeckStream,
};

type FirstResponse = Arc<Mutex<Option<Result<HttpResponseBasic, String>>>>;
type WrappedStream = NeckStream;
type StorageItem = (WrappedStream, FirstResponse);

type STORAGE = Arc<Mutex<HashMap<SocketAddr, StorageItem>>>;

pub(crate) struct Pool {
    storage: STORAGE,
}

pub enum ProxyResult {
    Ok(WrappedStream),
    BadGateway(),
    ServiceUnavailable(String),
}

impl Pool {
    pub fn new() -> Pool {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn len(&self) -> usize {
        self.storage.lock().await.len()
    }

    pub async fn join(&self, stream: NeckStream) {
        let shared_first_response: FirstResponse = Arc::new(Mutex::new(None));

        let am_first_response = shared_first_response.clone();
        let mut first_responses = am_first_response.lock().await;

        let addr = stream.peer_addr.clone();
        let am_reader = stream.reader.clone();

        self.storage
            .lock()
            .await
            .insert(addr, (stream, shared_first_response));

        *first_responses = Some(
            HttpResponseBasic::read_from(&mut *am_reader.lock().await)
                .await
                .map_err(|e| e.to_string()),
        );

        self.storage.lock().await.remove(&addr);
    }

    async fn take(&self) -> Option<StorageItem> {
        let mut map = self.storage.lock().await;
        let key = *map.keys().into_iter().next()?;
        map.remove(&key)
    }

    pub async fn connect(
        &self,
        // stream: &NeckStream,
        uri: &str,
    ) -> ProxyResult {
        // This is a retry loop, where certain operations can be retried, with a maximum of 5 retry attempts.
        for _ in 1..=5 {
            // Take a item from pool without retry.
            // If the pool is empty, retrying is pointless.
            let (stream, first_response) = match self.take().await {
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

            {
                // Read the first response from upstream.
                // This operation can be retryed.
                let first_response = first_response.lock().await;
                let res = match first_response.as_ref() {
                    Some(result) => match result {
                        Ok(res) => res,
                        Err(_) => {
                            continue;
                        }
                    },
                    None => {
                        continue;
                    }
                };

                // Got a non-200 status, this means proxy server cannot process this request, retrying is pointless.
                if res.get_status() != 200 {
                    return ProxyResult::ServiceUnavailable(res.get_payload().to_string());
                }
            }

            // Success, return the NeckStream object (transfer ownership).
            return ProxyResult::Ok(stream);
        }

        // After too many retry attempts, a 502 status response is respond.
        ProxyResult::BadGateway()
    }
}
