use std::{
    error::Error,
    fmt::{Debug, Display},
};

use serde::Deserialize;

/// A result type for olinker.
pub type Result<T> = std::result::Result<T, LlamaError>;

/// An error type for olinker.
#[derive(Deserialize)]
pub struct LlamaError {
    #[serde(rename = "error")]
    pub(crate) message: String,
}

impl Display for LlamaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "An error occurred with llama: {}", self.message)
    }
}

impl Debug for LlamaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "llama error: {}", self.message)
    }
}

impl Error for LlamaError {}

impl From<String> for LlamaError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<Box<dyn Error>> for LlamaError {
    fn from(error: Box<dyn Error>) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for LlamaError {
    fn from(error: serde_json::Error) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

impl From<reqwest::Error> for LlamaError {
    fn from(error: reqwest::Error) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}
