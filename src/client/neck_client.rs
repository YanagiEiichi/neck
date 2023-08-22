use std::{process::exit, sync::Arc};

use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

use crate::{neck::NeckStream, utils::NeckResult};

use super::{
    connector::Connector, neck_url::NeckUrl, start_worker::start_worker,
    token_bucket::TokenBucket,
};

use super::connector::TcpConnector;

#[cfg(feature = "tls")]
use super::connector::TlsConnector;

fn create_connector(
    url: &NeckUrl,
    #[allow(unused_variables)] tls_domain: Option<String>,
) -> Box<dyn Connector> {
    let tls = url.is_https();

    #[cfg(feature = "tls")]
    if tls {
        return Box::new(TlsConnector::new(url, tls_domain));
    }
    // If tls is enabled, but the tls feature is not enable, print an error message and exit the process.
    if tls {
        eprintln!("The 'https:' is not supported.");
        exit(1);
    }
    Box::new(TcpConnector::new(url))
}

pub struct NeckClient {
    pub url: NeckUrl,
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
        url: String,
        workers: Option<u32>,
        connections: Option<u32>,
        tls_domain: Option<String>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel::<Event>(32);

        let a = url.into();
        let connector = create_connector(&a, tls_domain);
        Self {
            url: a,
            // The number of concurrent workers defaults 8.
            workers: workers.unwrap_or(8),
            // Create a connector while considering the TLS configuration.
            connector,
            // Store the channel handler.
            sender,
            // The receiver is mutable, so wrap it with a Mutex to ensure the NeckClient remains immutable.
            receiver: Mutex::new(receiver),
            // The number of maximum provided connections defaults 200
            bucket: TokenBucket::new(connections.unwrap_or(200) as usize),
        }
    }

    /// Create a connect from connector.
    pub async fn connect(&self) -> NeckResult<NeckStream> {
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
                eprintln!("Failed to connect {}", self.url.get_addr());
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
