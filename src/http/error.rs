use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct HttpError(String);

impl HttpError {
    pub fn wrap<T>(message: impl ToString) -> Result<T, Box<dyn Error>> {
        Err(Box::new(HttpError(String::from(message.to_string()))))
    }
}

impl Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for HttpError {}
