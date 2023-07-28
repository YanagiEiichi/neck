use std::{error::Error, ops::Add, time::Duration};

use tokio::{io::AsyncWriteExt, net::TcpStream, time};

use crate::{
    http::{HttpRequest, HttpRequestBasic, HttpResponse, HttpResponseBasic},
    utils::{respond, respond_without_body, weld, NeckError},
};

async fn wait_until_http_proxy_connect(
    stream: &mut TcpStream,
) -> Result<HttpRequestBasic, Box<dyn Error>> {
    let req: HttpRequestBasic = HttpRequestBasic::read_from(stream).await?;
    if req.get_method().eq("CONNECT") {
        respond_without_body(stream, 200, "Connection Established", req.get_version()).await?;
        Ok(req)
    } else {
        respond(
            stream,
            405,
            "Method Not Allowed",
            req.get_version(),
            format!("Method '{}' not allowed\n", req.get_method()).as_str(),
        )
        .await?;
        Err(Box::new(NeckError::new(format!(
            "Bad HTTP method {}",
            req.get_method()
        ))))
    }
}

async fn connect_and_join(addr: &str) -> Result<TcpStream, Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    let req = HttpRequestBasic::new("JOIN", "*", "HTTP/1.1");
    stream.write(req.to_string().as_bytes()).await?;
    let res = HttpResponseBasic::read_from(&mut stream).await?;
    if res.get_status() == 200 {
        Ok(stream)
    } else {
        Err(Box::new(NeckError::new(format!(
            "Failed to join, get status {}",
            res.get_raw_status()
        ))))
    }
}

async fn setup_connection(addr: &str) -> Result<(), Box<dyn Error>> {
    let mut stream = connect_and_join(addr).await?;
    println!(
        "Connection {} ready",
        stream.local_addr().unwrap().to_string()
    );
    let req = wait_until_http_proxy_connect(&mut stream).await?;
    match TcpStream::connect(req.get_uri().as_str()).await {
        Ok(mut upstream) => {
            println!(
                "Connect to {} for {}",
                req.get_uri(),
                stream.local_addr().unwrap()
            );
            weld(&mut stream, &mut upstream).await;
            Ok(())
        }
        Err(e) => {
            respond(
                &mut stream,
                503,
                "Service Unavailable",
                req.get_version(),
                (e.to_string() + "\n").as_str(),
            )
            .await?;
            Err(Box::new(NeckError::new(format!(
                "Failed to connect {}",
                req.get_uri()
            ))))
        }
    }
}

async fn start_worker(addr: String) {
    let mut fails: u8 = 0;
    loop {
        match setup_connection(addr.as_str()).await {
            Ok(_) => {
                fails = 0;
            }
            Err(e) => {
                eprintln!("{}", e);
                fails = fails.add(1).min(6);
            }
        };
        if fails > 0 {
            time::sleep(Duration::from_secs(1 << (fails - 1))).await;
        }
    }
}

pub async fn start(addr: String, connections: u16) {
    let mut tasks = Vec::new();
    for _i in 1..=connections {
        tasks.push(tokio::spawn(start_worker(addr.clone())));
    }
    for task in tasks {
        tokio::select! {
          _ = task => ()
        }
    }
}
