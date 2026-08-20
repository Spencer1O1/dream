use thiserror::Error;

/// The invocation is invalid: wrong args or an unusable entry file.
#[derive(Debug, Error)]
#[error("UsageError: {error}")]
pub struct UsageError {
    error: String,
}

impl UsageError {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_error_string() {
        let err = UsageError::new("expected a .foo file");
        assert_eq!(err.to_string(), "UsageError: expected a .foo file");
    }
}
