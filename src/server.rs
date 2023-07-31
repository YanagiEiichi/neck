use std::{error::Error, sync::Arc};

use tokio::net::{TcpListener, TcpStream};

use crate::{
    http::{HttpRequest, HttpRequestBasic},
    pool::Pool,
    utils::{respond, respond_without_body, weld_for_rw},
};

async fn connect_handler(
    mut stream: TcpStream,
    req: &HttpRequestBasic,
    pool: Arc<Pool>,
) -> Result<(), Box<dyn Error>> {
    for _ in 1..=5 {
        match pool.take().await {
            Some(keeper) => {
                // Send CONNECT request to upstream first.
                match keeper.send_first_connect(req).await {
                    Ok((mr, mw)) => {
                        respond_without_body(
                            &mut stream,
                            200,
                            "Connection Established",
                            req.get_version(),
                        )
                        .await?;
                        println!(
                            "{} -> {}: CONNECT {}",
                            stream.peer_addr().unwrap().to_string(),
                            keeper.addr.to_string(),
                            req.get_uri()
                        );
                        let mut reader = mr.lock().await;
                        let mut writer = mw.lock().await;
                        weld_for_rw(&mut stream, &mut *reader, &mut *writer).await;
                        return Ok(());
                    }
                    Err(_) => {
                        // Bad keeper connection, can retry.
                        continue;
                    }
                }
            }
            None => {
                // Empty pool.
                break;
            }
        }
    }
    respond(
        &mut stream,
        502,
        "Bad Gateway",
        req.get_version(),
        "Connections are not available\n",
    )
    .await?;
    Ok(())
}

async fn join_handler(
    mut stream: TcpStream,
    req: &HttpRequestBasic,
    pool: Arc<Pool>,
) -> Result<(), Box<dyn Error>> {
    respond_without_body(&mut stream, 200, "Welcome", req.get_version()).await?;
    pool.join(stream).await;
    Ok(())
}

async fn get_handler(
    mut stream: TcpStream,
    req: &HttpRequestBasic,
    pool: Arc<Pool>,
) -> Result<(), Box<dyn Error>> {
    match req.get_uri().as_str() {
        "/pool/len" => {
            let payload = pool.len().await.to_string() + "\n";
            respond(&mut stream, 200, "OK", req.get_version(), &payload).await?;
        }
        _ => {
            respond(
                &mut stream,
                404,
                "Not Found",
                req.get_version(),
                "Not Found\n",
            )
            .await?;
        }
    }
    Ok(())
}

async fn reject_handler(
    mut stream: TcpStream,
    req: &HttpRequestBasic,
) -> Result<(), Box<dyn Error>> {
    respond(
        &mut stream,
        405,
        "Method Not Allowed",
        req.get_version(),
        format!("Method '{}' not allowed\n", req.get_method()).as_str(),
    )
    .await?;
    Ok(())
}

async fn dispatch(mut stream: TcpStream, pool: Arc<Pool>) {
    let req = HttpRequestBasic::read_from(&mut stream).await.unwrap();
    let res = match req.get_method().as_str() {
        "CONNECT" => connect_handler(stream, &req, pool).await,
        "JOIN" => join_handler(stream, &req, pool).await,
        "GET" => get_handler(stream, &req, pool).await,
        _ => reject_handler(stream, &req).await,
    };
    match res {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{:#}", e);
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
