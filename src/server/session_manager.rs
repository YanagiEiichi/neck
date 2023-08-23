use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU8, AtomicUsize, Ordering::SeqCst},
        Arc, Weak,
    },
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

enum Action {
    Insert(usize, Weak<RawSession>),
    Remove(usize),
}

type Storage = Arc<Mutex<BTreeMap<usize, Weak<RawSession>>>>;

pub struct SessionManager {
    inc: AtomicUsize,
    storage: Storage,
    sender: Sender<Action>,
}

async fn consumer_deamon(storage: Storage, mut receiver: Receiver<Action>) {
    while let Some(action) = receiver.recv().await {
        match action {
            Action::Insert(key, value) => {
                storage.lock().await.insert(key, value);
            }
            Action::Remove(key) => {
                if let Some(_value) = storage.lock().await.remove(&key) {
                    // println!("remove {}", value.host);
                };
            }
        }
    }
}

pub type Session = Arc<RawSession>;

#[derive(Debug, Serialize)]
pub struct RawSession {
    pub id: usize,
    pub timestamp: u128,
    pub proto: &'static str,
    pub host: String,
    pub from: SocketAddr,

    /// 0: Waiting, 1: Connecting, 2: Established.
    pub state: AtomicU8,

    #[serde(skip_serializing)]
    sender: Sender<Action>,
}

impl RawSession {
    pub fn set_it_connecting(&self) {
        self.state.store(1, SeqCst)
    }

    pub fn set_it_established(&self) {
        self.state.store(2, SeqCst)
    }
}

impl Drop for RawSession {
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
        let storage = Arc::new(Mutex::new(BTreeMap::<usize, Weak<RawSession>>::new()));
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

    /// Acqure an unique Id.
    fn create_id(&self) -> usize {
        self.inc.fetch_add(1, SeqCst)
    }

    /// Get current timestamp.
    fn now(&self) -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }

    pub fn create_session(&self, proto: &'static str, from: SocketAddr, host: String) -> Session {
        // Create the session.
        let session = Arc::new(RawSession {
            id: self.create_id(),
            state: AtomicU8::new(0),
            timestamp: self.now(),
            proto,
            host,
            from,
            sender: self.sender.clone(),
        });

        // Try to send the info.
        let _ = self
            .sender
            .try_send(Action::Insert(session.id, Arc::downgrade(&session)));

        session
    }

    pub async fn list(&self) -> Result<String, serde_json::Error> {
        let storage = self.storage.lock().await;
        let upgraded_list = storage
            .values()
            .filter_map(|v| v.upgrade())
            .collect::<Vec<Session>>();
        let ptr_list = upgraded_list
            .iter()
            .map(|v| v.as_ref())
            .collect::<Vec<&RawSession>>();
        serde_json::to_string(&ptr_list)
    }
}
