use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{atomic::AtomicUsize, Arc},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tokio::{
    spawn,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
};

use crate::neck::NeckStream;

#[derive(Debug, Serialize)]
pub struct ConnnectionInfo {
    key: usize,
    proto: &'static str,
    timestamp: u128,
    from: SocketAddr,
    to: String,
}

impl ConnnectionInfo {
    pub fn new(key: usize, proto: &'static str, from: SocketAddr, to: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        Self {
            key,
            proto,
            timestamp,
            from,
            to,
        }
    }
}

enum Action {
    Insert(ConnnectionInfo),
    Remove(usize),
}

type Storage = Arc<Mutex<BTreeMap<usize, ConnnectionInfo>>>;

pub struct SessionManager {
    inc: AtomicUsize,
    storage: Storage,
    sender: Sender<Action>,
}

async fn consumer_deamon(storage: Storage, mut receiver: Receiver<Action>) {
    while let Some(action) = receiver.recv().await {
        match action {
            Action::Insert(value) => {
                let key = value.key;
                println!("insert {}", value.to);
                storage.lock().await.insert(key, value);
            }
            Action::Remove(key) => {
                if let Some(value) = storage.lock().await.remove(&key) {
                    println!("remove {}", value.to);
                };
            }
        }
    }
}

pub struct Session {
  id: usize,
  sender: Sender<Action>,
}

impl Drop for Session {
    fn drop(&mut self) {
        // Try to send a remove message synchronously.
        if let Ok(_) = self.sender.try_send(Action::Remove(self.id)) {
            return;
        }
        // Otherwise, spawn a new routine to execute this action.
        let sender = self.sender.clone();
        let action = Action::Remove(self.id);
        spawn(async move {
            let _ = sender.send(action).await;
        });
    }
}

impl SessionManager {
    pub fn new() -> Self {
        let storage = Arc::new(Mutex::new(BTreeMap::<usize, ConnnectionInfo>::new()));
        let (sender, receiver) = channel(128);
        let inc = AtomicUsize::new(1);
        let mc = Self {
            inc,
            storage,
            sender,
        };
        spawn(consumer_deamon(mc.storage.clone(), receiver));
        mc
    }

    pub fn create_session(&self, name: &'static str, stream: &NeckStream, host: String) -> Session {
        // Acqure an unique Id.
        let id = self.inc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // Create the connection info.
        let value = ConnnectionInfo::new(id, name, stream.peer_addr, host);
        // Try to send the info.
        let _ = self.sender.try_send(Action::Insert(value));
        // Create the session.
        Session { sender: self.sender.clone(), id }
    }
}
