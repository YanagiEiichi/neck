use std::{borrow::Cow, error::Error, sync::Arc};

use tokio::net::TcpStream;

use crate::{
    http::{HttpCommon, HttpRequest},
    neck::NeckStream,
    utils::NeckError,
};

use super::{connection_manager::ConnectingResult, NeckServer};

async fn connect_upstream(
    stream: &NeckStream,
    host: &str,
    version: &str,
    ctx: &Arc<NeckServer>,
) -> Result<NeckStream, Box<dyn Error>> {
    match ctx.manager.connect(host.to_string()).await {
        ConnectingResult::Ok(v) => Ok(v),

        // Not enough available worker connections in the manager.
        ConnectingResult::BadGateway() => {
            println!(
                "[{}] No available connections for {}",
                stream.peer_addr.to_string(),
                host
            );
            stream
                .respond(
                    502,
                    "Bad Gateway",
                    version,
                    "Connections are not available\n",
                )
                .await?;
            stream.shutdown().await?;
            NeckError::wrap("Bad Gateway")
        }

        // Cannot establish a connection with the provided host.
        ConnectingResult::ServiceUnavailable(msg) => {
            println!(
                "[{}] Failed to connect {}",
                stream.peer_addr.to_string(),
                host
            );
            stream
                .respond(503, "Service Unavailable", version, &msg)
                .await?;
            stream.shutdown().await?;
            NeckError::wrap("Service Unavailable")
        }
    }
}

/// Process an HTTPS proxy request.
async fn connect_handler(
    stream: NeckStream,
    req: &HttpRequest,
    ctx: &Arc<NeckServer>,
) -> Result<(), Box<dyn Error>> {
    // Attempt to connect upstream server via the proxy connection manager.
    let upstream = connect_upstream(&stream, req.get_uri(), req.get_version(), ctx).await?;

    // Now, a successful connection has been established with the upstream server.

    println!(
        "[{}] Connect to {} for {}",
        stream.peer_addr.to_string(),
        upstream.peer_addr.to_string(),
        req.get_uri()
    );

    // Send a 200 Connection Established response to the client to answer the requested CONNECT method.
    stream
        .respond(200, "Connection Established", req.get_version(), "")
        .await?;

    // Weld the client connection with upstream.
    stream.weld(&upstream).await;

    Ok(())
}

async fn simple_http_proxy_handler(
    stream: NeckStream,
    req: &HttpRequest,
    ctx: &Arc<NeckServer>,
) -> Result<(), Box<dyn Error>> {
    // Remove "http://" from left
    let uri = &req.get_uri()[7..];

    // Split host and path.
    // For example:
    // "example.com/xxx" result ("example.com", "/xxx")
    // "example.com" result ("example.com", "/")
    let (mut host, path) = match uri.find('/') {
        Some(pos) => (Cow::Borrowed(&uri[..pos]), &uri[pos..]),
        None => (Cow::Borrowed(uri), "/"),
    };

    // Fix host (append a default HTTP port).
    if !host.contains(':') {
        host = Cow::Owned(format!("{}:80", host));
    }

    // Attempt to connect upstream server via the proxy connection manager.
    let upstream = connect_upstream(&stream, &host, req.get_version(), ctx).await?;

    // Now, a successful connection has been established with the upstream server.

    println!(
        "[{}] Connect to {} for http://{}",
        stream.peer_addr.to_string(),
        upstream.peer_addr.to_string(),
        host
    );

    // Remove Proxy-Connection header.
    let mut headers = req.get_headers().clone();
    headers.remove("Proxy-Connection");

    // Send an HTTP request (with the host part removed from original URI, leaving only the path part).
    upstream
        .request(req.get_method(), path, req.get_version(), headers)
        .await?;

    // Weld the client connection with upstream.
    stream.weld(&upstream).await;

    Ok(())
}

async fn join_handler(
    stream: NeckStream,
    req: &HttpRequest,
    ctx: &Arc<NeckServer>,
) -> Result<(), Box<dyn Error>> {
    // Respond a with 200 Welcome.
    stream
        .respond(200, "Welcome", req.get_version(), "")
        .await?;

    // Join the manager (ownership for the stream is moved to the manager)
    ctx.manager.join(stream).await;

    Ok(())
}

async fn api_handler(
    stream: NeckStream,
    req: &HttpRequest,
    ctx: &Arc<NeckServer>,
) -> Result<(), Box<dyn Error>> {
    let uri = req.get_uri();
    if uri.eq("/manager/len") && req.get_method().eq("GET") {
        let payload = ctx.manager.len().await.to_string() + "\n";
        stream
            .respond(200, "OK", req.get_version(), &payload)
            .await?;
    } else {
        stream
            .respond(404, "Not Found", req.get_version(), "Not Found\n")
            .await?;
    }
    Ok(())
}

pub async fn request_handler(tcp_stream: TcpStream, ctx: Arc<NeckServer>) {
    // Wrap the raw TcpStream with a NeckStream.
    let stream = NeckStream::from(tcp_stream);

    // Read the first request.
    // NOTE: Do not read payload here, because payload may be a huge stream.
    let req = match stream
        .read_http_request_header_only()
        .await
        .map_err(|e| e.to_string())
    {
        Ok(v) => v,
        Err(_) => {
            // Unable to read the HTTP request from the stream.
            // Exiting the thread early to terminate the connection (NeckStream will be Drop).
            return;
        }
    };

    // Dispatch to different handlers.
    match req.get_method() {
        "CONNECT" => connect_handler(stream, &req, &ctx).await,
        "JOIN" => join_handler(stream, &req, &ctx).await,
        _ => {
            // It is a simple HTTP proxy request.
            if req.get_uri().starts_with("http://") {
                simple_http_proxy_handler(stream, &req, &ctx).await
            }
            // Others.
            else {
                api_handler(stream, &req, &ctx).await
            }
        }
    }
    .unwrap_or_else(
        #[allow(unused_variables)]
        |e| {
            // println!("{:#?}", e);
        },
    )
}
