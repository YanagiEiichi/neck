use std::{error::Error, sync::Arc};

use tokio::net::{TcpListener, TcpStream};

use crate::{
    http::{HttpCommonBasic, HttpRequest, HttpRequestBasic},
    neck::NeckStream,
    pool::{Pool, ProxyResult},
};

async fn connect_handler(
    stream: NeckStream,
    req: &HttpRequestBasic,
    pool: Arc<Pool>,
) -> Result<(), Box<dyn Error>> {
    match pool.connect(req.get_uri()).await {
        ProxyResult::Ok(upstream) => {
            println!(
                "[{}] Connect to {} for {}",
                stream.peer_addr.to_string(),
                upstream.peer_addr.to_string(),
                req.get_uri()
            );
            stream
                .respond(200, "Connection Established", req.get_version(), "")
                .await?;
            stream.weld(&upstream).await;
        }
        ProxyResult::BadGateway() => {
            println!(
                "[{}] No available connections for {}",
                stream.peer_addr.to_string(),
                req.get_uri()
            );
            stream
                .respond(
                    502,
                    "Bad Gateway",
                    req.get_version(),
                    "Connections are not available\n",
                )
                .await?;
            stream.shutdown().await?;
        }
        ProxyResult::ServiceUnavailable(msg) => {
            println!(
                "[{}] Failed to connect {}",
                stream.peer_addr.to_string(),
                req.get_uri()
            );
            stream
                .respond(503, "Service Unavailable", req.get_version(), &msg)
                .await?;
            stream.shutdown().await?;
        }
    }
    Ok(())
}

async fn join_handler(
    stream: NeckStream,
    req: &HttpRequestBasic,
    pool: Arc<Pool>,
) -> Result<(), Box<dyn Error>> {
    stream
        .respond(200, "Welcome", req.get_version(), "")
        .await?;
    pool.join(stream).await;
    Ok(())
}

async fn api_handler(
    stream: NeckStream,
    req: &HttpRequestBasic,
    pool: Arc<Pool>,
) -> Result<(), Box<dyn Error>> {
    let uri = req.get_uri();
    if uri.eq("/pool/len") && req.get_method().eq("GET") {
        let payload = pool.len().await.to_string() + "\n";
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

async fn simple_http_proxy_handler(
    stream: NeckStream,
    req: &HttpRequestBasic,
    pool: Arc<Pool>,
) -> Result<(), Box<dyn Error>> {
    // Remove "http://" from left
    let uri = &req.get_uri()[7..];
    // Find the first slash
    if let Some(pos) = uri.find('/') {
        let mut host = String::from(&uri[..pos]);
        let path = &uri[pos..];

        if !host.contains(':') {
            host.push_str(":80");
        }

        match pool.connect(&host).await {
            ProxyResult::Ok(upstream) => {
                println!(
                    "[{}] Connect to {} for http://{}",
                    stream.peer_addr.to_string(),
                    upstream.peer_addr.to_string(),
                    host
                );
                let mut headers = req.get_headers().clone();
                headers.remove("Proxy-Connection");
                upstream
                    .write(
                        HttpRequestBasic::new(req.get_method(), path, req.get_version(), headers)
                            .to_string(),
                    )
                    .await?;
                stream.weld(&upstream).await;
            }
            ProxyResult::BadGateway() => {
                println!(
                    "[{}] No available connections for http://{}",
                    stream.peer_addr.to_string(),
                    host
                );
                stream
                    .respond(
                        502,
                        "Bad Gateway",
                        req.get_version(),
                        "Connections are not available\n",
                    )
                    .await?;
                stream.shutdown().await?;
            }
            ProxyResult::ServiceUnavailable(msg) => {
                println!(
                    "[{}] Failed to connect http://{}",
                    stream.peer_addr.to_string(),
                    host
                );
                stream
                    .respond(503, "Service Unavailable", req.get_version(), &msg)
                    .await?;
                stream.shutdown().await?;
            }
        }
    } else {
        stream
            .respond(400, "Bad Request", req.get_version(), "Bad URI")
            .await?;
    }
    Ok(())
}

async fn dispatch(tcp_stream: TcpStream, pool: Arc<Pool>) -> Result<(), String> {
    let stream = NeckStream::new(tcp_stream);

    // Read the first request.
    let req = stream
        .read_http_request()
        .await
        .map_err(|e| e.to_string())?;

    // Dispatch to different handlers.
    match req.get_method().as_str() {
        "CONNECT" => connect_handler(stream, &req, pool).await,
        "JOIN" => join_handler(stream, &req, pool).await,
        _ => {
            // It is a simple HTTP proxy request.
            if req.get_uri().starts_with("http://") {
                simple_http_proxy_handler(stream, &req, pool).await
            }
            // Others.
            else {
                api_handler(stream, &req, pool).await
            }
        }
    }
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Start a neck server.
pub async fn start(addr: String) {
    let shared_pool = Arc::new(Pool::new());

    match TcpListener::bind(addr).await {
        Ok(listener) => loop {
            let (socekt, _) = listener.accept().await.unwrap();
            let pool = shared_pool.clone();
            tokio::spawn(dispatch(socekt, pool));
        },
        Err(e) => {
            eprint!("{}", e);
        }
    };
}
