use thiserror::Error;

/// The dreamed program stopped. Produced by `dream_error` and the lucid turn cap. Not compose.
#[derive(Debug, Error)]
#[error("InterpreterError: {error}")]
pub struct InterpreterError {
    error: String,
}

impl InterpreterError {
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
        let err = InterpreterError::new(r#"Cannot assign input "2.5" to int x."#);
        assert_eq!(
            err.to_string(),
            r#"InterpreterError: Cannot assign input "2.5" to int x."#
        );
    }
}
