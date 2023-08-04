use std::{error::Error, sync::Arc};

use tokio::net::{TcpListener, TcpStream};

use crate::{
    http::{HttpCommonBasic, HttpRequest, HttpRequestBasic, HttpResponse},
    neck::NeckStream,
    pool::{Keeper, Pool},
    utils::NeckError,
};

async fn create_proxy_connection(
    stream: &NeckStream,
    req: &HttpRequestBasic,
    pool: &Arc<Pool>,
) -> Result<Arc<Keeper>, Box<dyn Error>> {
    // This is a retry loop, where certain operations can be retried, with a maximum of 5 retry attempts.
    for _ in 1..=5 {
        // Take a Keeper from pool without retry.
        // If the pool is empty, retrying is pointless.
        let keeper = match pool.take().await {
            Some(k) => k,
            None => {
                break;
            }
        };

        // Send the PROXY request to upstream.
        keeper
            .stream
            .write(req.to_string())
            .await
            .map_err(|e| e.to_string())?;

        {
            // Read the first response from upstream.
            // This operation can be retryed.
            let first_response = keeper.first_response.lock().await;
            let res = match first_response.as_ref() {
                Ok(res) => res,
                Err(_) => {
                    continue;
                }
            };

            // Got a non-200 status, this means proxy server cannot process this request, retrying is pointless.
            if res.get_status() != 200 {
                stream
                    .respond(503, res.get_text(), req.get_version(), res.get_payload())
                    .await?;
                stream.shutdown().await?;
                let message = format!(
                    "[{}] Faild to create connection with {} from {}",
                    stream.peer_addr().to_string(),
                    req.get_uri(),
                    keeper.stream.peer_addr().to_string(),
                );
                println!("{}", message);
                return Err(Box::new(NeckError::new(message)));
            }
        }

        println!(
            "[{}] Connect to {} host {}",
            stream.peer_addr().to_string(),
            keeper.stream.peer_addr().to_string(),
            req.get_uri()
        );
        return Ok(keeper);
    }
    stream
        .respond(
            502,
            "Bad Gateway",
            req.get_version(),
            "Connections are not available\n",
        )
        .await?;
    stream.shutdown().await?;
    let message = format!(
        "[{}] Connections are not available for host {}",
        stream.peer_addr().to_string(),
        req.get_uri()
    );
    println!("{}", message);
    Err(Box::new(NeckError::new(message)))
}

async fn connect_handler(
    stream: NeckStream,
    req: &HttpRequestBasic,
    pool: Arc<Pool>,
) -> Result<(), Box<dyn Error>> {
    let keeper = create_proxy_connection(&stream, req, &pool).await?;
    stream
        .respond(200, "Connection Established", req.get_version(), "")
        .await?;
    stream.weld(&keeper.stream).await;
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

async fn get_handler(
    stream: NeckStream,
    req: &HttpRequestBasic,
    pool: Arc<Pool>,
) -> Result<(), Box<dyn Error>> {
    match req.get_uri().as_str() {
        "/pool/len" => {
            let payload = pool.len().await.to_string() + "\n";
            stream
                .respond(200, "OK", req.get_version(), &payload)
                .await?;
        }
        _ => {
            stream
                .respond(404, "Not Found", req.get_version(), "Not Found\n")
                .await?;
        }
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

        let c_req = HttpRequestBasic::new("CONNECT", &host, "HTTP/1.1", vec![]);
        let keeper = create_proxy_connection(&stream, &c_req, &pool).await?;

        let mut headers = req.get_headers().clone();
        headers.remove("Proxy-Connection");
        let b_req = HttpRequestBasic::new(req.get_method(), path, req.get_version(), headers);
        keeper.stream.write(b_req.to_string()).await?;

        stream.weld(&keeper.stream).await;
    } else {
        stream
            .respond(400, "Bad Request", req.get_version(), "Bad URI")
            .await?;
    }
    Ok(())
}

async fn reject_handler(stream: NeckStream, req: &HttpRequestBasic) -> Result<(), Box<dyn Error>> {
    stream
        .respond(
            405,
            "Method Not Allowed",
            req.get_version(),
            format!("Method '{}' not allowed\n", req.get_method()).as_str(),
        )
        .await?;
    Ok(())
}

async fn dispatch(tcp_stream: TcpStream, pool: Arc<Pool>) -> Result<(), String> {
    let stream = NeckStream::new(tcp_stream);
    let req = stream
        .read_http_request()
        .await
        .map_err(|e| e.to_string())?;
    match req.get_method().as_str() {
        "CONNECT" => connect_handler(stream, &req, pool).await,
        "JOIN" => join_handler(stream, &req, pool).await,
        _ => {
            // It's a simple HTTP proxy request.
            if req.get_uri().starts_with("http://") {
                simple_http_proxy_handler(stream, &req, pool).await
            } else if req.get_method().eq("GET") {
                get_handler(stream, &req, pool).await
            } else {
                reject_handler(stream, &req).await
            }
        }
    }
    .map_err(|e| e.to_string())?;
    Ok(())
}

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
