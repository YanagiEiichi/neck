use std::{error::Error, sync::Arc};

use tokio::net::{TcpListener, TcpStream};

use crate::{
    http::{HttpRequest, HttpRequestBasic, HttpResponse},
    neck::NeckStream,
    pool::Pool,
};

async fn connect_handler(
    stream: NeckStream,
    req: &HttpRequestBasic,
    pool: Arc<Pool>,
) -> Result<(), Box<dyn Error>> {
    'end: {
        for _ in 1..=5 {
            let o = pool.take().await;
            if o.is_none() {
                break;
            }

            let keeper = o.unwrap();

            keeper
                .stream
                .write(req.to_string())
                .await
                .map_err(|e| e.to_string())?;

            let first_response = keeper.first_response.lock().await;
            if first_response.is_err() {
                continue;
            }

            let res = first_response.as_ref().unwrap();

            if let 200 = res.get_status() {
                stream
                    .respond(200, "Connection Established", req.get_version(), "")
                    .await?;
                println!(
                    "[{}] Connect to {} host {}",
                    stream.peer_addr().to_string(),
                    keeper.stream.peer_addr().to_string(),
                    req.get_uri()
                );
                keeper.stream.weld(&stream).await;
            } else {
                stream
                    .respond(503, res.get_text(), req.get_version(), res.get_payload())
                    .await?;
                stream.shutdown().await?;
                println!(
                    "[{}] Faild to create connection with {} from {}",
                    stream.peer_addr().to_string(),
                    req.get_uri(),
                    keeper.stream.peer_addr().to_string(),
                );
            }
            break 'end;
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
        println!(
            "[{}] Connections are not available for host {}",
            stream.peer_addr().to_string(),
            req.get_uri()
        );
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

async fn dispatch(tcp_stream: TcpStream, pool: Arc<Pool>) {
    let stream = NeckStream::new(tcp_stream);
    let first_request = stream.read_http_request().await.map_err(|e| e.to_string());
    let peer_addr = stream.peer_addr().to_string();
    if let Ok(req) = first_request {
        let res = match req.get_method().as_str() {
            "CONNECT" => connect_handler(stream, &req, pool).await,
            "JOIN" => join_handler(stream, &req, pool).await,
            "GET" => get_handler(stream, &req, pool).await,
            _ => reject_handler(stream, &req).await,
        };
        match res {
            Ok(_) => (),
            Err(e) => {
                eprintln!("[{}] {}", peer_addr, e.to_string());
            }
        }
    }
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
