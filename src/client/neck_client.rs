use std::{process::exit, sync::Arc};

use tokio::{
    io,
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
};

use crate::neck::NeckStream;

use super::{
    connector::Connector, start_worker::start_worker, tcp_connector::TcpConnector,
    token_bucket::TokenBucket,
};

#[cfg(feature = "tls")]
use super::tls_connector::TlsConnector;

fn create_connector(
    addr: String,
    tls_enabled: bool,
    #[allow(unused_variables)] tls_domain: Option<String>,
) -> Box<dyn Connector> {
    #[cfg(feature = "tls")]
    if tls_enabled {
        return Box::new(TlsConnector::new(addr, tls_domain));
    }
    // If tls is enabled, but the tls feature is not enable, print an error message and exit the process.
    if tls_enabled {
        eprintln!("The '--tls' flag is not supported.");
        exit(1);
    }
    Box::new(TcpConnector::new(addr))
}

pub struct NeckClient {
    pub addr: String,
    pub workers: u32,
    pub bucket: TokenBucket,
    connector: Box<dyn Connector>,
    sender: Sender<Event>,
    receiver: Mutex<Receiver<Event>>,
}

pub enum Event {
    Joined,
    Failed,
}

impl NeckClient {
    pub fn new(
        addr: String,
        workers: Option<u32>,
        connections: Option<u32>,
        tls_enabled: bool,
        tls_domain: Option<String>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel::<Event>(32);
        Self {
            addr: addr.clone(),
            // The number of concurrent workers defaults 8.
            workers: workers.unwrap_or(8),
            // Create a connector while considering the TLS configuration.
            connector: create_connector(addr, tls_enabled, tls_domain),
            // Store the channel handler.
            sender,
            // The receiver is mutable, so wrap it with a Mutex to ensure the NeckClient remains immutable.
            receiver: Mutex::new(receiver),
            // The number of maximum provided connections defaults 200
            bucket: TokenBucket::new(connections.unwrap_or(200) as usize),
        }
    }

    /// Create a connect from connector.
    pub async fn connect(&self) -> io::Result<NeckStream> {
        self.connector.connect().await
    }

    /// Dispatch an event.
    pub async fn dispatch_event(&self, event: Event) {
        let _ = self.sender.send(event).await;
    }

    /// Wait and process events.
    async fn wait(&self) {
        let mut receiver = self.receiver.lock().await;

        // Initialize some counters.
        let mut failed_count = 0u32;

        // Read event from channel.
        while let Some(event) = receiver.recv().await {
            match event {
                // If anyone successfully joins the server, reset the failed counter.
                Event::Joined => failed_count = 0,
                // Increase the failed counter.
                Event::Failed => failed_count += 1,
            }
            // If the failed counter exceeds the number of connections, print an error message.
            if failed_count > self.workers {
                eprintln!("Failed to connect {}", self.addr);
                // Reset failed counter to debounce the error message printing.
                failed_count = 0;
            }
        }
    }

    /// Start workers.
    pub async fn start(self) {
        // Wrap ctx with Arc, it will be used in all child threads.
        let shared_ctx = Arc::new(self);

        // Create threads for each client connection.
        for _ in 0..shared_ctx.workers {
            tokio::spawn(start_worker(shared_ctx.clone()));
        }

        // Wait and process events.
        shared_ctx.wait().await;
    }
}
