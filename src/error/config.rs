use thiserror::Error;

/// Dream cannot start: environment or settings are invalid.
#[derive(Debug, Error)]
#[error("ConfigError: {error}")]
pub struct ConfigError {
    error: String,
}

impl ConfigError {
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
        let err = ConfigError::new("OPENAI_API_KEY is not set");
        assert_eq!(err.to_string(), "ConfigError: OPENAI_API_KEY is not set");
    }
}
