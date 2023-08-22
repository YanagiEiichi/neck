use std::sync::Arc;

use crate::{
    http::{HttpRequest, HttpResponse},
    neck::NeckStream,
    utils::NeckResult,
};

use super::{
    super::NeckServer,
    api::api_handler,
    join::join_handler,
    proxy::{http_proxy_handler, https_proxy_handler},
};

pub async fn http_handler(stream: NeckStream, ctx: Arc<NeckServer>) -> NeckResult<()> {
    // Read the first request.
    // NOTE: Do not read payload here, because payload may be a huge stream.
    let req = HttpRequest::read_header_from(&stream).await?;

    // Dispatch to different handlers.
    if let "CONNECT" = req.get_method() {
        https_proxy_handler(stream, &req, &ctx).await
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
        http_proxy_handler(stream, &req, &ctx).await
    }
    // Others.
    else {
        api_handler(stream, &req, &ctx).await
    }
}
