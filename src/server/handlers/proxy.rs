use std::{borrow::Cow, sync::Arc};

use crate::{
    http::{HttpCommon, HttpRequest, HttpResponse},
    neck::NeckStream,
    server::session_manager::Session,
    utils::{NeckError, NeckResult},
};

use super::super::{manager::ConnectingResult, NeckServer};

async fn connect_upstream(
    stream: &NeckStream,
    session: &Session,
    version: &str,
    ctx: &Arc<NeckServer>,
) -> NeckResult<NeckStream> {
    match ctx.manager.connect(session).await {
        ConnectingResult::Ok(v) => Ok(v),

        // Not enough available worker connections in the manager.
        ConnectingResult::BadGateway() => {
            println!(
                "[{}] No available connections for {}",
                stream.peer_addr.to_string(),
                session.host
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
                session.host
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
pub async fn https_proxy_handler(
    stream: NeckStream,
    req: &HttpRequest,
    ctx: &Arc<NeckServer>,
) -> NeckResult<()> {
    let session =
        ctx.session_manager
            .create_session("https", stream.peer_addr, req.get_uri().to_string());

    // Attempt to connect upstream server via the proxy connection manager.
    let upstream = connect_upstream(&stream, &session, req.get_version(), ctx).await?;

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

    drop(session);

    Ok(())
}

pub async fn http_proxy_handler(
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

    let session = ctx
        .session_manager
        .create_session("http", stream.peer_addr, host.to_string());

    // Attempt to connect upstream server via the proxy connection manager.
    let upstream = connect_upstream(&stream, &session, req.get_version(), ctx).await?;

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

    drop(session);

    Ok(())
}
