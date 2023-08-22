use std::{process::exit, sync::Arc};

use tokio::net::TcpListener;

use crate::utils::BoxedError;

use super::{
    handlers::request_handler,
    manager::{ConnectionManager, DirectModeManager, PoolModeManager},
};

fn fix_addr(addr: Option<String>) -> String {
    addr.map_or_else(
        // Get addr, use "0.0.0.0:1081" as the default valeu.
        || String::from("0.0.0.0:1081"),
        // Convert pure number {port} to "0.0.0.0:{port}"
        |v| v.parse::<u16>().map_or(v, |i| format!("0.0.0.0:{}", i)),
    )
}

fn error_handler(e: BoxedError) {
    #[cfg(debug_assertions)]
    println!("{:#?}", e);
}

pub struct NeckServer {
    pub addr: String,
    pub manager: Box<dyn ConnectionManager>,
}

impl NeckServer {
    /// Creates a new [`ServerContext`].
    pub fn new(addr: Option<String>, direct: bool, max_workers: Option<u32>) -> Self {
        Self {
            addr: fix_addr(addr),
            manager: if direct {
                Box::new(DirectModeManager {})
            } else {
                // The maximum allowed number of workers defaults 200.
                Box::new(PoolModeManager::new(max_workers.unwrap_or(200) as usize))
            },
        }
    }

    /// Start a neck server.
    pub async fn start(self) -> ! {
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
                    let ctx = shared_ctx.clone();
                    tokio::spawn(async move {
                        // Wrap the raw TcpStream with a NeckStream.
                        request_handler(stream.into(), ctx)
                            .await
                            .unwrap_or_else(error_handler);
                    });
                }
                Err(e) => {
                    eprint!("{}", e);
                }
            };
        }
    }
}
