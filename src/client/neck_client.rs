use std::{error::Error, process::exit, sync::Arc};

use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

use crate::neck::NeckStream;

use super::{connector::Connector, start_worker::start_worker, tcp_connector::TcpConnector};

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
    pub connections: u64,
    connector: Box<dyn Connector>,
    sender: Sender<Event>,
    receiver: Mutex<Receiver<Event>>,
}

enum Event {
    Joined,
    Failed,
}

impl NeckClient {
    pub fn new(
        addr: String,
        connections: Option<u64>,
        tls_enabled: bool,
        tls_domain: Option<String>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel::<Event>(32);
        Self {
            addr: addr.clone(),
            // The connections is defaults 100
            connections: connections.unwrap_or(100),
            // Create a connector while considering the TLS configuration.
            connector: create_connector(addr, tls_enabled, tls_domain),
            // Store the channel handler.
            sender,
            // The receiver is mutable, so wrap it with a Mutex to ensure the NeckClient remains immutable.
            receiver: Mutex::new(receiver),
        }
    }

    /// Create a connect from connector.
    pub async fn connect(&self) -> Result<NeckStream, Box<dyn Error>> {
        self.connector.connect().await
    }

    /// Fire a joind event.
    pub async fn fire_joined_event(&self) {
        let _ = self.sender.send(Event::Joined).await;
    }

    /// Fire a failed event.
    pub async fn fire_failed_event(&self) {
        let _ = self.sender.send(Event::Failed).await;
    }

    /// Wait and process events.
    async fn wait(&self) {
        let mut receiver = self.receiver.lock().await;

        // Initialize some counters.
        let mut failed_count = 0u64;

        // Read event from channel.
        while let Some(event) = receiver.recv().await {
            match event {
                // If anyone successfully joins the server, reset the failed counter.
                Event::Joined => failed_count = 0,
                // Increase the failed counter.
                Event::Failed => failed_count += 1,
            }
            // If the failed counter exceeds the number of connections, print an error message.
            if failed_count > self.connections {
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
        for _ in 0..shared_ctx.connections {
            tokio::spawn(start_worker(shared_ctx.clone()));
        }

        // Wait and process events.
        shared_ctx.wait().await;
    }
}
