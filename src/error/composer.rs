use thiserror::Error;

/// Composition, lock, repair, or build stopped.
#[derive(Debug, Error)]
#[error("ComposerError: {error}")]
pub struct ComposerError {
    error: String,
}

impl ComposerError {
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
        let err = ComposerError::new("locked unit `main.foo` source changed");
        assert_eq!(
            err.to_string(),
            "ComposerError: locked unit `main.foo` source changed"
        );
    }
}
