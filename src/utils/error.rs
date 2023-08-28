use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct NeckError(String);

impl NeckError {
    pub fn new(msg: impl ToString) -> Self {
        NeckError(msg.to_string())
    }
}

impl Display for NeckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for NeckError {}

pub type BoxedError = Box<dyn Error + Send + Sync>;
