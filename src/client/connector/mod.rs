mod connector_tcp;

#[cfg(feature = "tls")]
mod connector_tls;

use std::{future::Future, pin::Pin};

use crate::{neck::NeckStream, utils::NeckResult};

pub use connector_tcp::*;

#[cfg(feature = "tls")]
pub use connector_tls::*;

pub type ConnResult<'a> = Pin<Box<dyn Future<Output = NeckResult<NeckStream>> + Send + 'a>>;

pub trait Connector: Send + Sync {
    fn connect(&self) -> ConnResult<'_>;
}
