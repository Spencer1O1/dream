use std::fmt;
use std::io;

/// Run-level failure. Printed as `DreamError: ...`.
#[derive(Debug)]
pub struct DreamError {
    message: String,
}

impl DreamError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DreamError: {}", self.message)
    }
}

impl std::error::Error for DreamError {}

impl From<io::Error> for DreamError {
    fn from(err: io::Error) -> Self {
        Self::new(err.to_string())
    }
}
