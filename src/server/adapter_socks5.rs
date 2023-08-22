use std::sync::Arc;

use crate::{
    neck::NeckStream,
    socks5::{ClientGreeting, ServerChoice, Sock5Connection},
    utils::{NeckError, NeckResult},
};

use super::{manager::ConnectingResult, NeckServer};

pub async fn sock5_handler(stream: NeckStream, ctx: Arc<NeckServer>) -> NeckResult<()> {
    let addr = read_sock5_request(&stream).await?;

    match ctx.manager.connect(addr.clone()).await {
        ConnectingResult::Ok(upstream) => {
            println!(
                "[{}] Connect to {} for {} [socks5]",
                stream.peer_addr.to_string(),
                upstream.peer_addr.to_string(),
                addr
            );
            Sock5Connection::new(0).write_to_stream(&stream).await?;

            // Weld the client connection with upstream.
            stream.weld(&upstream).await;
        }
        ConnectingResult::BadGateway() => {
            println!(
                "[{}] No available connections for {}",
                stream.peer_addr.to_string(),
                addr
            );
            Sock5Connection::new(1).write_to_stream(&stream).await?;
        }
        ConnectingResult::ServiceUnavailable(_) => {
            println!(
                "[{}] Failed to connect {}",
                stream.peer_addr.to_string(),
                addr
            );
            Sock5Connection::new(1).write_to_stream(&stream).await?;
        }
    };

    Ok(())
}

async fn read_sock5_request(stream: &NeckStream) -> NeckResult<String> {
    let mut reader = stream.reader.lock().await;
    let mut writer = stream.writer.lock().await;

    // Read a socks5 ClientGreeting reqeuest.
    let greeting = ClientGreeting::read_from(&mut reader).await?;

    // Forcing the use of the 0 value, regardless of whether the ClientGreeting supports.
    ServerChoice::new(greeting.ver, 0)
        .write_to(&mut *writer)
        .await?;

    let req = Sock5Connection::read_from(&mut reader).await?;
    // println!("{:#?}", req);

    if req.action != 1 {
        Sock5Connection::new(7).write_to(&mut *writer).await?;
        NeckError::wrap("Unsupported socks5 cmd")?
    }

    Ok(req.to_addr())
}
