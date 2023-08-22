use std::sync::Arc;

use tokio::{io::AsyncBufReadExt, net::TcpStream};

use crate::neck::NeckStream;

use super::{http_adapter::http_handler, sock5_adapter::sock5_handler, NeckServer};

pub async fn is_socks5(stream: &NeckStream) -> bool {
    // Wait and peek the first buffer to check if the first byte is 5u8.
    match stream.reader.lock().await.fill_buf().await {
        Ok(v) => v.first().map_or(false, |x| *x == 5),
        Err(_) => false,
    }
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
