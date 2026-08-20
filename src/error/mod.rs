//! Process-level failure. Printed as the subtype that produced it.
//!
//! | Type | When |
//! |------|------|
//! | [`UsageError`] | This invocation does not apply: flags, not a `.foo`, dest occupied, lock with no store. |
//! | [`ConfigError`] | Dream cannot start: environment or settings. |
//! | [`InterpreterError`] | The dreamed program stopped (`dream_error`, lucid turn cap). Not compose. |
//! | [`ComposerError`] | Composition, lock, repair, or build stopped. |
//! | [`RuntimeError`] | Host plumbing that is neither the program nor compose: OpenAI, I/O, JSON, a corrupt store. Shared helpers that do not know the session. |
//!
//! Tool refusals use [`detail`](DreamError::detail) only. The subtype prefix is for process abort.

mod composer;
mod config;
mod interpreter;
mod runtime;
mod usage;

use std::io;

use thiserror::Error;

pub use composer::ComposerError;
pub use config::ConfigError;
pub use interpreter::InterpreterError;
pub use runtime::RuntimeError;
pub use usage::UsageError;

#[derive(Debug, Error)]
pub enum DreamError {
    #[error("{0}")]
    Interpreter(#[from] InterpreterError),
    #[error("{0}")]
    Composer(#[from] ComposerError),
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

    pub fn composer(error: impl Into<String>) -> Self {
        ComposerError::new(error).into()
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

    pub fn detail(&self) -> &str {
        match self {
            Self::Interpreter(err) => err.detail(),
            Self::Composer(err) => err.detail(),
            Self::Runtime(err) => err.detail(),
            Self::Config(err) => err.detail(),
            Self::Usage(err) => err.detail(),
        }
    }
}

impl From<io::Error> for DreamError {
    fn from(err: io::Error) -> Self {
        Self::runtime(err.to_string())
    }
}

impl From<serde_json::Error> for DreamError {
    fn from(err: serde_json::Error) -> Self {
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
            DreamError::composer("locked unit `main.foo` source changed").to_string(),
            "ComposerError: locked unit `main.foo` source changed"
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
