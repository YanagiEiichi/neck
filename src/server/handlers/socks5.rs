use std::sync::Arc;

use crate::{
    neck::NeckStream,
    socks5::{ClientGreeting, ServerChoice, Socks5Message},
    utils::{NeckError, NeckResult},
};

use super::super::{manager::ConnectingResult, NeckServer};

pub async fn sock5_handler(stream: NeckStream, ctx: Arc<NeckServer>) -> NeckResult<()> {
    let req = read_sock5_request(&stream).await?;

    let session =
        ctx.session_manager
            .create_session("sock5", stream.peer_addr, req.host.to_string());

    match ctx.manager.connect(&session).await {
        ConnectingResult::Ok(upstream) => {
            println!(
                "[{}] Connect to {} for {} [socks5]",
                stream.peer_addr.to_string(),
                upstream.peer_addr.to_string(),
                req.host.to_string()
            );
            req.clone().set_action(0).write_to_stream(&stream).await?;

            // Weld the client connection with upstream.
            stream.weld(&upstream).await;
        }
        ConnectingResult::BadGateway() => {
            println!(
                "[{}] No available connections for {}",
                stream.peer_addr.to_string(),
                req.host.to_string()
            );
            req.clone().set_action(1).write_to_stream(&stream).await?;
        }
        ConnectingResult::ServiceUnavailable(_) => {
            println!(
                "[{}] Failed to connect {}",
                stream.peer_addr.to_string(),
                req.host.to_string()
            );
            req.clone().set_action(1).write_to_stream(&stream).await?;
        }
    };

    drop(session);

    Ok(())
}

async fn read_sock5_request(stream: &NeckStream) -> NeckResult<Socks5Message> {
    let mut reader = stream.reader.lock().await;
    let mut writer = stream.writer.lock().await;

    // Read a socks5 ClientGreeting reqeuest.
    let greeting = ClientGreeting::read_from(&mut reader).await?;

    // Forcing the use of the 0 value, regardless of whether the ClientGreeting supports.
    ServerChoice::new(greeting.ver, 0)
        .write_to(&mut *writer)
        .await?;

    let req = Socks5Message::read_from(&mut reader).await?;
    // println!("{:#?}", req);

    if req.action != 1 {
        req.clone().set_action(7).write_to(&mut *writer).await?;
        NeckError::wrap("Unsupported socks5 cmd")?
    }

    Ok(req)
}
