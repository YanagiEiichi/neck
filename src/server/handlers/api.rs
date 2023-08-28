use std::sync::Arc;

use crate::{
    http::{HttpRequest, HttpResponse},
    utils::{NeckResult, NeckStream},
};

use super::super::NeckServer;

pub async fn api_handler(
    stream: NeckStream,
    req: &HttpRequest,
    ctx: &Arc<NeckServer>,
) -> NeckResult<()> {
    let uri = req.get_uri();
    if uri.eq("/api/len") && req.get_method().eq("GET") {
        HttpResponse::new(200, "OK", req.get_version())
            .add_payload(ctx.manager.len().await.to_string().as_bytes())
            .add_payload(b"\n")
            .write_to_stream(&stream)
            .await?;
    } else if uri.eq("/api/mc") && req.get_method().eq("GET") {
        HttpResponse::new(200, "OK", req.get_version())
            .add_payload(ctx.session_manager.list().await.unwrap().as_bytes())
            .add_header("Content-Type: application/json")
            .write_to_stream(&stream)
            .await?;
    } else if uri.eq("/dashboard") && req.get_method().eq("GET") {
        HttpResponse::new(200, "OK", req.get_version())
            .add_payload(include_bytes!("../../static/index.html"))
            .add_header("Content-Type: text/html")
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
