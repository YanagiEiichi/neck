use std::{error::Error, future::Future, pin::Pin};

use crate::neck::NeckStream;

pub type ConnResult<'a> =
    Pin<Box<dyn Future<Output = Result<NeckStream, Box<dyn Error>>> + Send + 'a>>;

pub trait Connector: Send + Sync {
    fn connect(&self) -> ConnResult<'_>;
}
