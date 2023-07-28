use std::{collections::HashMap, error::Error, net::SocketAddr, sync::Arc};

use tokio::{
    io::AsyncWriteExt,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
    task::JoinHandle,
};

use crate::http::{HttpRequestBasic, HttpResponse, HttpResponseBasic};

type STORAGE = Arc<Mutex<HashMap<SocketAddr, Keeper>>>;

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

    pub async fn insert(&self, stream: TcpStream) {
        let keeper = Keeper::new(stream, self.storage.clone());
        self.storage.lock().await.insert(keeper.addr, keeper);
    }

    pub async fn take(&self) -> Option<Keeper> {
        let mut map = self.storage.lock().await;
        let key = *map.keys().into_iter().next()?;
        map.remove(&key)
    }
}

pub struct Keeper {
    pub addr: SocketAddr,
    pub heap: Option<(
        OwnedWriteHalf,
        JoinHandle<(OwnedReadHalf, Result<HttpResponseBasic, String>)>,
    )>,
}

impl Keeper {
    pub fn new(stream: TcpStream, storage: STORAGE) -> Keeper {
        let addr = stream.peer_addr().unwrap();
        let (mut reader, writter) = stream.into_split();

        let first_response: JoinHandle<(OwnedReadHalf, Result<HttpResponseBasic, String>)> =
            tokio::spawn(async move {
                let res = match HttpResponseBasic::read_from(&mut reader).await {
                    Ok(res) => Ok(res),
                    Err(e) => Err(e.to_string()),
                };
                storage.lock().await.remove(&addr);
                (reader, res)
            });

        Self {
            addr,
            heap: Some((writter, first_response)),
        }
    }

    async fn inner_send_first_connect(
        &mut self,
        req: &HttpRequestBasic,
    ) -> Result<
        (
            OwnedWriteHalf,
            OwnedReadHalf,
            Result<HttpResponseBasic, String>,
        ),
        Box<dyn Error>,
    > {
        let (mut writer, first_response) = self.heap.take().unwrap();
        writer.write(req.to_string().as_bytes()).await?;
        let (reader, result) = first_response.await?;
        Ok((writer, reader, result))
    }

    pub async fn send_first_connect(
        &mut self,
        req: &HttpRequestBasic,
    ) -> Result<(OwnedReadHalf, OwnedWriteHalf), String> {
        match self.inner_send_first_connect(req).await {
            Ok((writer, reader, result)) => match result {
                Ok(res) => {
                    if res.get_status() == 200 {
                        Ok((reader, writer))
                    } else {
                        Err(format!("Fail to CONNECT, got {}", res.get_status()))
                    }
                }
                Err(msg) => Err(msg),
            },
            Err(e) => Err(e.to_string()),
        }
    }
}
