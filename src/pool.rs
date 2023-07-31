use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::{
    io::AsyncWriteExt,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
};

use crate::http::{HttpRequestBasic, HttpResponse, HttpResponseBasic};

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

    pub async fn join(&self, stream: TcpStream) {
        let shared_keeper = Arc::new(Keeper::new(stream));
        let keeper = shared_keeper.clone();
        self.storage.lock().await.insert(shared_keeper.addr, shared_keeper);

        let mut read = keeper.r.lock().await;
        let mut first_responses = keeper.first_response.lock().await;
        *first_responses = match HttpResponseBasic::read_from(&mut *read).await {
            Ok(res) => Ok(res),
            Err(e) => Err(e.to_string()),
        };

        self.storage.lock().await.remove(&keeper.addr);
    }

    pub async fn take(&self) -> Option<Arc<Keeper>> {
        let mut map = self.storage.lock().await;
        let key = *map.keys().into_iter().next()?;
        map.remove(&key)
    }
}

pub struct Keeper {
    pub addr: SocketAddr,
    pub w: Arc<Mutex<OwnedWriteHalf>>,
    pub r: Arc<Mutex<OwnedReadHalf>>,
    pub first_response: Arc<Mutex<Result<HttpResponseBasic, String>>>,
}

impl Keeper {
    pub fn new(stream: TcpStream) -> Keeper {
        let addr = stream.peer_addr().unwrap();
        let (orh, owh) = stream.into_split();

        let r = Arc::new(Mutex::new(orh));
        let w = Arc::new(Mutex::new(owh));
        let first_response = Arc::new(Mutex::new(Err(String::new())));

        Self {
            addr,
            w,
            r,
            first_response
        }
    }

    pub async fn send_first_connect(
        &self,
        req: &HttpRequestBasic,
    ) -> Result<(&Mutex<OwnedReadHalf>, &Mutex<OwnedWriteHalf>), String> {
        self.w.lock().await.write(req.to_string().as_bytes()).await.map_err(|e| e.to_string())?;

        let first_response = self.first_response.lock().await;

        match &*first_response {
            Ok(res) => {
                if res.get_status() == 200 {
                    Ok((&self.r, &self.w))
                } else {
                    Err(format!("Fail to CONNECT, got {}", res.get_status()))
                }
            }
            Err(msg) => Err(msg.clone()),
        }
    }
}
