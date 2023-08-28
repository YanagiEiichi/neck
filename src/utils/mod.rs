use std::{future::Future, pin::Pin};

mod error;
mod stream;

pub use error::*;
pub use stream::*;

/// PBF = Pin Box Future
pub type PBF<'a, O> = Pin<Box<dyn Future<Output = O> + Send + 'a>>;

pub type NeckResult<T> = Result<T, BoxedError>;

impl NeckError {
    pub fn wrap<T>(message: impl ToString) -> NeckResult<T> {
        Err(Box::new(NeckError::new(message)))
    }
}
