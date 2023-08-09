use std::{error::Error, ops::Add, sync::Arc, time::Duration};

use tokio::{net::TcpStream, spawn, time};

use crate::{http::HttpRequest, neck::NeckStream, utils::NeckError};

async fn wait_until_http_proxy_connect(stream: &NeckStream) -> Result<HttpRequest, Box<dyn Error>> {
    // Attempt to read a HTTP request.
    let req = stream.read_http_request().await?;

    // If method is "CONNECT" return the `req` directly.
    if req.get_method().eq("CONNECT") {
        return Ok(req);
    }

    // Otherwise, respond with a 405 status code.
    stream
        .respond(
            405,
            "Method Not Allowed",
            req.get_version(),
            format!("Method '{}' not allowed\n", req.get_method()).as_str(),
        )
        .await?;

    // And return a standard error object.
    NeckError::wrap(format!("Bad HTTP method {}", req.get_method()))
}

/// Create a connection and try to join the NeckServer.
async fn connect_and_join(ctx: &ClientContext) -> Result<NeckStream, Box<dyn Error>> {
    // Attempt to connect NeckServer.
    let stream = ctx.connect().await?;

    // Attempt to send a JOIN request.
    stream.request("JOIN", "*", "HTTP/1.1", vec![]).await?;

    // Attempt to read the corresponding response of the JOIN request above.
    let res = stream.read_http_response().await?;

    // Return the stream object if a 200 status code received.
    if res.get_status() == 200 {
        return Ok(stream);
    }

    // Otherwise, return a standard error object.
    NeckError::wrap(format!("Failed to join, get status {}", res.get_status()))
}

async fn setup_connection(ctx: &ClientContext) -> Result<(), Box<dyn Error>> {
    // Create a connection and try to join the NeckServer.
    let stream = connect_and_join(ctx).await?;

    // Wait for any received CONNECT requests.
    let req = wait_until_http_proxy_connect(&stream).await?;

    // Attempt to connect the upstream server.
    match TcpStream::connect(req.get_uri()).await {
        // If the connection is established successfully.
        Ok(upstream) => {
            println!("[{}] Connect to {}", stream.local_addr, req.get_uri());

            // Answer the CONNECT request
            stream
                .respond(200, "Connection Established", req.get_version(), "")
                .await?;

            // Weld the client connection with upstream.
            stream.weld(&NeckStream::from(upstream)).await;

            Ok(())
        }
        // Cannot connect to upstream server.
        Err(e) => {
            println!("[{}] Faild to connect {}", stream.local_addr, req.get_uri());

            // Answer a 503 status.
            stream
                .respond(
                    503,
                    "Service Unavailable",
                    req.get_version(),
                    (e.to_string() + "\n").as_str(),
                )
                .await?;

            NeckError::wrap(format!("Failed to connect {}", req.get_uri()))
        }
    }
}

async fn start_worker(ctx: Arc<ClientContext>) {
    // Initialize a failure counter.
    let mut failures: u8 = 0;

    loop {
        failures = match setup_connection(&ctx).await {
            // Reset failure counter if the taks success.
            Ok(_) => 0,
            // Increase the failure counter (maximum of 6).
            #[allow(unused_variables)]
            Err(e) => {
                // eprintln!("{}", e.to_string());
                failures.add(1).min(6)
            }
        };
        // If the failure counter is not zero, sleep for a few seconds (following exponential backoff).
        if failures > 0 {
            time::sleep(Duration::from_secs(1 << (failures - 1))).await;
        }
    }
}

pub struct ClientContext {
    pub addr: String,
    pub connections: Option<u16>,
    tls: Option<(tokio_native_tls::TlsConnector, String)>,
}

impl ClientContext {
    pub fn new(
        addr: String,
        connections: Option<u16>,
        tls_enabled: bool,
        tls_domain: Option<String>,
    ) -> Self {
        // Create tls context only when tls is enabled.
        let tls = tls_enabled.then(|| {
            (
                // Initialize the TlsConnector
                native_tls::TlsConnector::new().unwrap().into(),
                // If tls_domain is not set, get the hostname from addr.
                tls_domain.unwrap_or_else(|| addr.split(':').next().unwrap().to_string()),
            )
        });

        Self {
            addr,
            connections,
            tls,
        }
    }

    pub async fn connect(&self) -> Result<NeckStream, Box<dyn Error>> {
        // Attempt to connect Neck Server.
        let tcp_stream = TcpStream::connect(&self.addr).await?;

        // Connect NeckServer (may over TLS)
        let stream: NeckStream = match self.tls.as_ref() {
            Some((connector, domain)) => {
                let peer_addr = tcp_stream.peer_addr().unwrap();
                let local_addr = tcp_stream.local_addr().unwrap();
                NeckStream::new(
                    peer_addr,
                    local_addr,
                    connector.connect(domain, tcp_stream).await?,
                )
            }
            None => {
                // Wrap the connection with NeckStream
                tcp_stream.into()
            }
        };
        Ok(stream)
    }
}

pub async fn start(ctx: ClientContext) {
    // Wrap ctx with Arc, it will be used in all child threads.
    let shared_ctx = Arc::new(ctx);

    // The connections is defaults 100
    let connections = shared_ctx.connections.unwrap_or(100);

    // Create threads for each client connection.
    let tasks: Vec<_> = (0..connections)
        .map(|_| spawn(start_worker(shared_ctx.clone())))
        .collect();

    // Wait for all tasks to be completed.
    // Although in reality, none of them will be done, as they are running indefinitely.
    for task in tasks {
        let _ = task.await;
    }
}
