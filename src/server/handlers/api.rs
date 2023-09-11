use std::sync::Arc;

use tokio::io::AsyncWriteExt;

use crate::{
    http::{HttpRequest, HttpResponse},
    utils::{NeckResult, NeckStream},
};

use super::super::{static_manager::get_static_matcher, NeckServer};

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
    } else if uri.eq("/api/sessions") && req.get_method().eq("GET") {
        HttpResponse::new(200, "OK", req.get_version())
            .add_payload(ctx.session_manager.list().await.unwrap().as_bytes())
            .add_header("Content-Type: application/json")
            .write_to_stream(&stream)
            .await?;
    } else if uri.eq("/api/events") && req.get_method().eq("GET") {
        HttpResponse::new(200, "OK", req.get_version())
            .add_header("Content-Type: text/event-stream")
            .write_to_stream(&stream)
            .await?;
        let mut id = 1;
        stream
            .writer
            .lock()
            .await
            .write_all(format!("id: {}\nevent: init\ndata: null\n\n", id).as_bytes())
            .await?;
        loop {
            ctx.session_manager.watch().await;
            stream
                .writer
                .lock()
                .await
                .write_all(format!("id: {}\nevent: update\ndata: null\n\n", id).as_bytes())
                .await?;
            id += 1;
        }
    } else if req.get_method().eq("GET") {
        get_static_matcher().execute(req, &stream).await?;
    } else {
        HttpResponse::new(405, "Not Allowed", req.get_version())
            .add_payload(b"Not Allowed\n")
            .add_header("Cache-Control: no-cache")
            .write_to_stream(&stream)
            .await?;
    }
    Ok(())
}
