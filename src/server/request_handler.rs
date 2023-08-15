use std::{borrow::Cow, error::Error, sync::Arc};

use tokio::net::TcpStream;

use crate::{
    http::{HttpCommon, HttpRequest, HttpResponse},
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

            HttpResponse::new(502, "Bad Gateway", version)
                .add_payload(b"Connections are not available\n")
                .write_to_stream(stream)
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

            HttpResponse::new(503, "Service Unavailable", version)
                .add_payload(msg.as_bytes())
                .write_to_stream(stream)
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
    HttpResponse::new(200, "Connection Established", req.get_version())
        .write_to_stream(&stream)
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

    // Send an HTTP request (with the host part removed from original URI, leaving only the path part).
    let mut m_req = HttpRequest::new(req.get_method(), path, req.get_version());

    // Copy headers excluding Proxy-Connection.
    for h in req.get_headers().iter() {
        if !h.eq_name("Proxy-Connection") {
            m_req.headers.push(h.clone());
        }
    }

    m_req.write_to_stream(&upstream).await?;

    // Weld the client connection with upstream.
    stream.weld(&upstream).await;

    Ok(())
}

async fn join_handler(
    stream: NeckStream,
    req: &HttpRequest,
    ctx: &Arc<NeckServer>,
) -> Result<(), Box<dyn Error>> {
    // Respond a status with 101 Switching Protocols.
    HttpResponse::new(101, "Switching Protocols", req.get_version())
        .add_header("Connection: Upgrade")
        .add_header("Upgrade: neck")
        .write_to_stream(&stream)
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
        HttpResponse::new(200, "OK", req.get_version())
            .add_payload(ctx.manager.len().await.to_string().as_bytes())
            .add_payload(b"\n")
            .write_to_stream(&stream)
            .await?;
    } else {
        HttpResponse::new(404, "Not Found", req.get_version())
            .add_payload(b"Not Found\n")
            .write_to_stream(&stream)
            .await?;
    }
    Ok(())
}

pub async fn request_handler(tcp_stream: TcpStream, ctx: Arc<NeckServer>) {
    // Wrap the raw TcpStream with a NeckStream.
    let stream = NeckStream::from(tcp_stream);

    // Read the first request.
    // NOTE: Do not read payload here, because payload may be a huge stream.
    let req = match HttpRequest::read_header_from(&stream)
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
    if let "CONNECT" = req.get_method() {
        connect_handler(stream, &req, &ctx).await
    } else
    // For HTTP Upgrade.
    if let Some(upgrade) = req.headers.get_header("Upgrade") {
        if upgrade.eq("neck") {
            join_handler(stream, &req, &ctx).await
        } else {
            HttpResponse::new(400, "Bad Request", req.get_version())
                .add_payload(format!("The protocol '{}' is not supported.", upgrade).as_bytes())
                .write_to_stream(&stream)
                .await
        }
    } else
    // It is a simple HTTP proxy request.
    if req.get_uri().starts_with("http://") {
        simple_http_proxy_handler(stream, &req, &ctx).await
    }
    // Others.
    else {
        api_handler(stream, &req, &ctx).await
    }
    // Error handler.
    .unwrap_or_else(
        #[allow(unused_variables)]
        |e| {
            // println!("{:#?}", e);
        },
    )
}
