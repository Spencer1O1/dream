use crate::error::DreamError;

#[derive(Debug)]
pub enum Outcome {
    Ok,
    NoToolchain,
    MissingToolchain(String),
    Failed {
        step: &'static str,
        diagnostics: String,
    },
}

impl Outcome {
    pub fn into_error(self) -> Result<(), DreamError> {
        match self {
            Self::Ok => Ok(()),
            Self::NoToolchain => Err(DreamError::composer(
                "Dream does not know how to build this target",
            )),
            Self::MissingToolchain(hint) => Err(DreamError::composer(hint)),
            Self::Failed { step, .. } => Err(DreamError::composer(format!("{step} failed"))),
        }
    }
}
