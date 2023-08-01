use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct NeckError {
    message: String,
}

impl NeckError {
    pub fn new(message: String) -> NeckError {
        NeckError { message }
    }
}

impl Display for NeckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for NeckError {}
