use std::sync::Arc;

use tokio::io::AsyncBufReadExt;

use crate::{neck::NeckStream, utils::NeckResult, server::NeckServer};

use super::{http::http_handler, socks5::sock5_handler};

pub async fn is_socks5(stream: &NeckStream) -> bool {
  // Wait and peek the first buffer to check if the first byte is 5u8.
  match stream.reader.lock().await.fill_buf().await {
      Ok(v) => v.first().map_or(false, |x| *x == 5),
      Err(_) => false,
  }
}

pub async fn request_handler(stream: NeckStream, ctx: Arc<NeckServer>) -> NeckResult<()> {
  if is_socks5(&stream).await {
    sock5_handler(stream, ctx).await
  } else {
    http_handler(stream, ctx).await
  }
}