use crate::{neck::NeckStream, utils::NeckResult, utils::PBF};

mod connector_tcp;
pub use connector_tcp::*;

#[cfg(feature = "tls")]
mod connector_tls;
#[cfg(feature = "tls")]
pub use connector_tls::*;

pub type ConnResult<'a> = PBF<'a, NeckResult<NeckStream>>;

pub trait Connector: Send + Sync {
    fn connect(&self) -> ConnResult<'_>;
}
