use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use tokio::sync::Notify;

pub struct TokenBucket {
    value: Arc<AtomicUsize>,
    notify: Arc<Notify>,
}

pub struct Token(Arc<AtomicUsize>, Arc<Notify>);

impl Drop for Token {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
        self.1.notify_one();
    }
}

impl TokenBucket {
    pub async fn acquire(&self) -> Token {
        while self
            .value
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                if v > 0 {
                    Some(v - 1)
                } else {
                    None
                }
            })
            .is_err()
        {
            self.notify.notified().await;
        }
        Token(self.value.clone(), self.notify.clone())
    }

    pub fn new(size: usize) -> TokenBucket {
        Self {
            value: Arc::new(AtomicUsize::new(size)),
            notify: Arc::new(Notify::new()),
        }
    }
}
