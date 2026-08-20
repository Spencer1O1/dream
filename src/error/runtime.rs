use thiserror::Error;

/// Dream failed while doing work: source, tools, I/O, or the model API.
#[derive(Debug, Error)]
#[error("RuntimeError: {error}")]
pub struct RuntimeError {
    error: String,
}

impl RuntimeError {
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
        let err = RuntimeError::new("source request escapes project root");
        assert_eq!(
            err.to_string(),
            "RuntimeError: source request escapes project root"
        );
    }
}
