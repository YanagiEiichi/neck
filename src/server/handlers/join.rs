use std::sync::Arc;

use crate::{
    http::{HttpRequest, HttpResponse},
    utils::{NeckResult, NeckStream},
};

use super::super::NeckServer;

pub async fn join_handler(
    stream: NeckStream,
    req: &HttpRequest,
    ctx: &Arc<NeckServer>,
) -> NeckResult<()> {
    // Respond a status with 101 Switching Protocols.
    HttpResponse::new(101, "Switching Protocols", req.get_version())
        .add_header("Connection: Upgrade")
        .add_header("Upgrade: neck")
        .write_to_stream(&stream)
        .await?;

    // Join the manager (ownership for the stream is moved to the manager)
    ctx.manager.join(stream).await;

    Ok(())
}
