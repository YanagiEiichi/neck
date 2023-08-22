use std::{borrow::Cow, sync::Arc};

use tokio::{io::AsyncBufReadExt, net::TcpStream};

use crate::{
    http::{HttpCommon, HttpRequest, HttpResponse},
    neck::NeckStream,
    socks5::{ClientGreeting, ServerChoice, Sock5Connection},
    utils::{NeckError, NeckResult},
};

use super::{connection_manager::ConnectingResult, NeckServer};

async fn connect_upstream(
    stream: &NeckStream,
    host: &str,
    version: &str,
    ctx: &Arc<NeckServer>,
) -> NeckResult<NeckStream> {
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
) -> NeckResult<()> {
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
) -> NeckResult<()> {
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
) -> NeckResult<()> {
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
) -> NeckResult<()> {
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

pub async fn is_socks5(stream: &NeckStream) -> bool {
    // Wait and peek the first buffer to check if the first byte is 5u8.
    match stream.reader.lock().await.fill_buf().await {
        Ok(v) => v.first().map_or(false, |x| *x == 5),
        Err(_) => false,
    }
}

async fn http_handler(stream: NeckStream, ctx: Arc<NeckServer>) -> NeckResult<()> {
    // Read the first request.
    // NOTE: Do not read payload here, because payload may be a huge stream.
    let req = HttpRequest::read_header_from(&stream).await?;

    // Dispatch to different handlers.
    if let "CONNECT" = req.get_method() {
        connect_handler(stream, &req, &ctx).await
    } else
    // For HTTP Upgrade.
    if let Some(upgrade) = req.headers.get_header_value("Upgrade") {
        if upgrade.eq("neck") {
            join_handler(stream, &req, &ctx).await
        } else {
            HttpResponse::new(400, "Bad Request", req.get_version())
                .add_payload(format!("The protocol '{}' is not supported.", upgrade).as_bytes())
                .write_to_stream(&stream)
                .await
                .map_err(|e| e.into())
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
}

async fn sock5_handler(stream: NeckStream, ctx: Arc<NeckServer>) -> NeckResult<()> {
    let addr = read_sock5_request(&stream).await?;

    match ctx.manager.connect(addr.clone()).await {
        ConnectingResult::Ok(upstream) => {
            println!(
                "[{}] Connect to {} for {} [socks5]",
                stream.peer_addr.to_string(),
                upstream.peer_addr.to_string(),
                addr
            );
            Sock5Connection::new(0).write_to_stream(&stream).await?;

            // Weld the client connection with upstream.
            stream.weld(&upstream).await;
        }
        ConnectingResult::BadGateway() => {
            println!(
                "[{}] No available connections for {}",
                stream.peer_addr.to_string(),
                addr
            );
            Sock5Connection::new(1).write_to_stream(&stream).await?;
        }
        ConnectingResult::ServiceUnavailable(_) => {
            println!(
                "[{}] Failed to connect {}",
                stream.peer_addr.to_string(),
                addr
            );
            Sock5Connection::new(1).write_to_stream(&stream).await?;
        }
    };

    Ok(())
}

async fn read_sock5_request(stream: &NeckStream) -> NeckResult<String> {
    let mut reader = stream.reader.lock().await;
    let mut writer = stream.writer.lock().await;

    let hello = ClientGreeting::read_from(&mut reader).await?;

    ServerChoice::new(hello.ver, 0)
        .write_to(&mut *writer)
        .await?;

    let req = Sock5Connection::read_from(&mut reader).await?;
    // println!("{:#?}", req);

    if req.action != 1 {
        Sock5Connection::new(7).write_to_stream(stream).await?;
        NeckError::wrap("Unsupported socks5 cmd")?
    }

    Ok(req.to_addr())
}

pub async fn request_handler(tcp_stream: TcpStream, ctx: Arc<NeckServer>) {
    // Wrap the raw TcpStream with a NeckStream.
    let stream = NeckStream::from(tcp_stream);

    // Detect the protocol, and dispatch to the corresponding handler.
    if is_socks5(&stream).await {
        sock5_handler(stream, ctx).await
    } else {
        http_handler(stream, ctx).await
    }
    // Global error handler.
    .unwrap_or_else(
        #[allow(unused_variables)]
        |e| {
            // println!("{:#?}", e);
        },
    );
}
