use std::{error::Error, ops::Add, sync::Arc, time::Duration};

use tokio::{net::TcpStream, time};

use crate::{
    http::{HttpRequest, HttpResponse},
    neck::NeckStream,
    utils::NeckError,
};

use super::{Event::*, NeckClient};

async fn wait_until_http_proxy_connect(stream: &NeckStream) -> Result<HttpRequest, Box<dyn Error>> {
    // Attempt to read a HTTP request.
    let req = HttpRequest::read_from(stream).await?;

    // If method is "CONNECT" return the `req` directly.
    if req.get_method().eq("CONNECT") {
        return Ok(req);
    }

    // Otherwise, respond with a 405 status code.
    HttpResponse::new(405, "Method Not Allowed", req.get_version())
        .add_payload(format!("Method '{}' not allowed\n", req.get_method()).as_bytes())
        .write_to_stream(stream)
        .await?;

    // And return a standard error object.
    NeckError::wrap(format!("Bad HTTP method {}", req.get_method()))
}

/// Create a connection and try to join the NeckServer.
async fn connect_and_join(ctx: &NeckClient) -> Result<NeckStream, Box<dyn Error>> {
    // Attempt to connect NeckServer.
    let stream = ctx.connect().await?;

    // Attempt to send a request with Upgrade: neck.
    HttpRequest::new("GET", "/", "HTTP/1.1")
        .add_header_kv("Host", &ctx.addr)
        .add_header("Connection: Upgrade")
        .add_header("Upgrade: neck")
        .write_to_stream(&stream)
        .await?;

    // Attempt to read the corresponding response of the JOIN request above.
    let res = HttpResponse::read_from(&stream).await?;

    // Return the stream object if a 200 status code received.
    if res.get_status() == 101 {
        // Tell master, this connection has joined.
        ctx.dispatch_event(Joined).await;

        // Return the connected stream.
        return Ok(stream);
    }

    // Otherwise, return a standard error object.
    NeckError::wrap(format!("Failed to join, get status {}", res.get_status()))
}

async fn connect_upstream_and_weld(
    stream: &NeckStream,
    req: &HttpRequest,
) -> Result<(), Box<dyn Error>> {
    // Attempt to connect the upstream server.
    match TcpStream::connect(req.get_uri()).await {
        // If the connection is established successfully.
        Ok(upstream) => {
            println!("[{}] Connect to {}", stream.local_addr, req.get_uri());

            // Answer the CONNECT request
            HttpResponse::new(200, "Connection Established", req.get_version())
                .write_to_stream(&stream)
                .await?;

            // Weld stream and upstream toggle.
            stream.weld(&NeckStream::from(upstream)).await;
            Ok(())
        }
        // Cannot connect to upstream server.
        Err(e) => {
            println!("[{}] Faild to connect {}", stream.local_addr, req.get_uri());

            // Answer a 503 status.
            HttpResponse::new(503, "Service Unavailable", req.get_version())
                .add_payload(e.to_string().as_bytes())
                .add_payload(b"\n")
                .write_to_stream(&stream)
                .await?;

            NeckError::wrap(format!("Failed to connect {}", req.get_uri()))
        }
    }
}

async fn setup_connection(ctx: &NeckClient) -> Result<(), Box<dyn Error>> {
    let token = ctx.bucket.acquire().await;

    // Create a connection and try to join the NeckServer.
    let stream = connect_and_join(ctx).await?;

    // Wait for any received CONNECT requests.
    let req = wait_until_http_proxy_connect(&stream).await?;

    // If a CONNECT request is received, spawn a new asynchronous routine to handle subsequent matters.
    // The current routine should be released to handle the next requests.
    tokio::spawn(async move {
        // Attempt to connect the upstream server and weld, some io exceptions will be ignored here.
        if let Err(_) = connect_upstream_and_weld(&stream, &req).await {
            // There is nothing to handle here, as the above function has taken care of everything.
        }

        // The token will be held until this routine is complete, therefore drop it manually at this point.
        drop(token);
    });

    Ok(())
}

pub async fn start_worker(ctx: Arc<NeckClient>) {
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
            ctx.dispatch_event(Failed).await;
            time::sleep(Duration::from_secs(1 << (failures - 1))).await;
        }
    }
}
