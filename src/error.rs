use std::fmt;
use std::io;

/// Run-level failure. Printed as `DreamError: {error}`.
#[derive(Debug)]
pub struct DreamError {
    error: String,
}

impl DreamError {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

impl fmt::Display for DreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DreamError: {}", self.error)
    }
}

impl std::error::Error for DreamError {}

impl From<io::Error> for DreamError {
    fn from(err: io::Error) -> Self {
        Self::new(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_error_string() {
        let err = DreamError::new(r#"Cannot assign input "2.5" to int x."#);
        assert_eq!(
            err.to_string(),
            r#"DreamError: Cannot assign input "2.5" to int x."#
        );
    }
}
