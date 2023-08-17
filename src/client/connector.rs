use std::{future::Future, pin::Pin};

use tokio::io;

use crate::neck::NeckStream;

pub type ConnResult<'a> = Pin<Box<dyn Future<Output = io::Result<NeckStream>> + Send + 'a>>;

pub trait Connector: Send + Sync {
    fn connect(&self) -> ConnResult<'_>;
}
