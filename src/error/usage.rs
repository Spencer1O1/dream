use thiserror::Error;

/// This invocation does not apply: flags, not a `.foo`, dest occupied, lock with no store.
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

    pub fn detail(&self) -> &str {
        &self.error
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
