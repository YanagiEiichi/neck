use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct HttpError(String);

impl HttpError {
    pub fn new(message: impl ToString) -> Self {
        Self(message.to_string())
    }
    pub fn wrap<T>(message: impl ToString) -> Result<T, Box<dyn Error>> {
        Err(Box::new(Self::new(message)))
    }
}

impl Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for HttpError {}
