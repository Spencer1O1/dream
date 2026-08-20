use std::fmt;
use std::io;

/// Process-level failure. Printed as the subtype that produced it.
#[derive(Debug)]
pub enum DreamError {
    Interpreter(InterpreterError),
    Runtime(RuntimeError),
}

impl DreamError {
    pub fn interpreter(error: impl Into<String>) -> Self {
        Self::Interpreter(InterpreterError::new(error))
    }

    pub fn runtime(error: impl Into<String>) -> Self {
        Self::Runtime(RuntimeError::new(error))
    }
}

impl fmt::Display for DreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interpreter(err) => write!(f, "{err}"),
            Self::Runtime(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for DreamError {}

impl From<io::Error> for DreamError {
    fn from(err: io::Error) -> Self {
        Self::runtime(err.to_string())
    }
}

/// The dreamed program cannot continue. Produced by `dream_error`.
#[derive(Debug)]
pub struct InterpreterError {
    error: String,
}

impl InterpreterError {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InterpreterError: {}", self.error)
    }
}

/// Dream itself failed: source, tools, I/O, config, the model API.
#[derive(Debug)]
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

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RuntimeError: {}", self.error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_interpreter_error() {
        let err = DreamError::interpreter(r#"Cannot assign input "2.5" to int x."#);
        assert_eq!(
            err.to_string(),
            r#"InterpreterError: Cannot assign input "2.5" to int x."#
        );
    }

    #[test]
    fn formats_runtime_error() {
        let err = DreamError::runtime("OPENAI_API_KEY is not set");
        assert_eq!(err.to_string(), "RuntimeError: OPENAI_API_KEY is not set");
    }
}
