use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct NeckError(String);

type BoxedError = Box<dyn Error + Send + Sync>;

pub type NeckResult<T> = Result<T, BoxedError>;

impl NeckError {
    pub fn wrap<T>(message: impl ToString) -> NeckResult<T> {
        Err(Box::new(NeckError(String::from(message.to_string()))))
    }
}

impl Display for NeckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for NeckError {}
