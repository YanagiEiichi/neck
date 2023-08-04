use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    http::{HttpCommonBasic, HttpRequestBasic, HttpResponse, HttpResponseBasic},
    neck::NeckStream,
};

type STORAGE = Arc<Mutex<HashMap<SocketAddr, Arc<Keeper>>>>;

pub(crate) struct Pool {
    storage: STORAGE,
}

pub enum ProxyResult {
    Ok(Arc<Keeper>),
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
        let shared_keeper = Arc::new(Keeper::new(stream));

        let keeper = shared_keeper.clone();
        let mut first_responses = keeper.first_response.lock().await;

        self.storage
            .lock()
            .await
            .insert(*shared_keeper.stream.peer_addr(), shared_keeper);

        *first_responses = keeper
            .stream
            .read_http_response()
            .await
            .map_err(|e| e.to_string());

        self.storage.lock().await.remove(keeper.stream.peer_addr());
    }

    async fn take(&self) -> Option<Arc<Keeper>> {
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
            // Take a Keeper from pool without retry.
            // If the pool is empty, retrying is pointless.
            let keeper = match self.take().await {
                Some(k) => k,
                None => {
                    break;
                }
            };

            // Send the PROXY request to upstream.
            // This operation can be retryed.
            match keeper
                .stream
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
                let first_response = keeper.first_response.lock().await;
                let res = match first_response.as_ref() {
                    Ok(res) => res,
                    Err(_) => {
                        continue;
                    }
                };

                // Got a non-200 status, this means proxy server cannot process this request, retrying is pointless.
                if res.get_status() != 200 {
                    return ProxyResult::ServiceUnavailable(res.get_payload().to_string());
                    // stream
                    //     .respond(503, , req.get_version(), )
                    //     .await?;
                    // stream.shutdown().await?;
                    // let message = format!(
                    //     "[{}] Faild to create connection with {} from {}",
                    //     stream.peer_addr().to_string(),
                    //     req.get_uri(),
                    //     keeper.stream.peer_addr().to_string(),
                    // );
                    // println!("{}", message);
                    // return Err(Box::new(NeckError::new(message)));
                }
            }

            // Success, return the keeper object (transfer ownership).
            return ProxyResult::Ok(keeper);
        }

        // After too many retry attempts, a 502 status response is respond.
        ProxyResult::BadGateway()
    }
}

pub struct Keeper {
    pub stream: NeckStream,
    pub first_response: Mutex<Result<HttpResponseBasic, String>>,
}

impl Keeper {
    pub fn new(stream: NeckStream) -> Keeper {
        Self {
            stream,
            first_response: Mutex::new(Err(String::new())),
        }
    }
}
