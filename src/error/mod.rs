mod config;
mod interpreter;
mod runtime;
mod usage;

use std::io;

use thiserror::Error;

pub use config::ConfigError;
pub use interpreter::InterpreterError;
pub use runtime::RuntimeError;
pub use usage::UsageError;

/// Process-level failure. Printed as the subtype that produced it.
#[derive(Debug, Error)]
pub enum DreamError {
    #[error("{0}")]
    Interpreter(#[from] InterpreterError),
    #[error("{0}")]
    Runtime(#[from] RuntimeError),
    #[error("{0}")]
    Config(#[from] ConfigError),
    #[error("{0}")]
    Usage(#[from] UsageError),
}

impl DreamError {
    pub fn interpreter(error: impl Into<String>) -> Self {
        InterpreterError::new(error).into()
    }

    pub fn runtime(error: impl Into<String>) -> Self {
        RuntimeError::new(error).into()
    }

    pub fn config(error: impl Into<String>) -> Self {
        ConfigError::new(error).into()
    }

    pub fn usage(error: impl Into<String>) -> Self {
        UsageError::new(error).into()
    }
}

impl From<io::Error> for DreamError {
    fn from(err: io::Error) -> Self {
        Self::runtime(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegates_display_to_subtype() {
        assert_eq!(
            DreamError::interpreter(r#"Cannot assign input "2.5" to int x."#).to_string(),
            r#"InterpreterError: Cannot assign input "2.5" to int x."#
        );
        assert_eq!(
            DreamError::runtime("source request escapes project root").to_string(),
            "RuntimeError: source request escapes project root"
        );
        assert_eq!(
            DreamError::config("OPENAI_API_KEY is not set").to_string(),
            "ConfigError: OPENAI_API_KEY is not set"
        );
        assert_eq!(
            DreamError::usage("expected a .foo file").to_string(),
            "UsageError: expected a .foo file"
        );
    }
}
