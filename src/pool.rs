use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::sync::Mutex;

use crate::{http::HttpResponseBasic, neck::NeckStream};

type STORAGE = Arc<Mutex<HashMap<SocketAddr, Arc<Keeper>>>>;

pub(crate) struct Pool {
    storage: STORAGE,
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

    pub async fn take(&self) -> Option<Arc<Keeper>> {
        let mut map = self.storage.lock().await;
        let key = *map.keys().into_iter().next()?;
        map.remove(&key)
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
