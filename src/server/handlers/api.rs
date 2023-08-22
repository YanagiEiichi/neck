use std::sync::Arc;

use crate::{
    http::{HttpRequest, HttpResponse},
    neck::NeckStream,
    utils::NeckResult,
};

use super::super::NeckServer;

pub async fn api_handler(
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
