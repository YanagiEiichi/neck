use std::{future::Future, pin::Pin};

use crate::{neck::NeckStream, utils::NeckResult};

pub type ConnResult<'a> = Pin<Box<dyn Future<Output = NeckResult<NeckStream>> + Send + 'a>>;

pub trait Connector: Send + Sync {
    fn connect(&self) -> ConnResult<'_>;
}
