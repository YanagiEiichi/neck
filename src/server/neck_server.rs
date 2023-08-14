use std::{process::exit, sync::Arc};

use tokio::net::TcpListener;

use super::{
    connection_manager::ConnectionManager, direct_mode_manager::DirectModeManager,
    pool_mode_manager::PoolModeManager, request_handler::request_handler,
};

pub struct NeckServer {
    pub addr: String,
    pub manager: Box<dyn ConnectionManager>,
}

fn fix_addr(addr: Option<String>) -> String {
    addr.map_or_else(
        // Get addr, use "0.0.0.0:1081" as the default valeu.
        || String::from("0.0.0.0:1081"),
        // Convert pure number {port} to "0.0.0.0:{port}"
        |v| v.parse::<u16>().map_or(v, |i| format!("0.0.0.0:{}", i)),
    )
}

impl NeckServer {
    /// Creates a new [`ServerContext`].
    pub fn new(addr: Option<String>, direct: bool) -> Self {
        Self {
            addr: fix_addr(addr),
            manager: if direct {
                Box::new(DirectModeManager {})
            } else {
                Box::new(PoolModeManager::new())
            },
        }
    }

    /// Start a neck server.
    pub async fn start(self) {
        let shared_ctx = Arc::new(self);

        // Begin TCP listening on specified address.
        let listener = match TcpListener::bind(&shared_ctx.addr).await {
            Ok(v) => v,
            Err(e) => {
                eprint!("{}", e);
                exit(1);
            }
        };

        loop {
            // Accept all requests and dispatch each of them using a new thread.
            match listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(request_handler(stream, shared_ctx.clone()));
                }
                Err(e) => {
                    eprint!("{}", e);
                }
            };
        }
    }
}
