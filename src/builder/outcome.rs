use crate::error::DreamError;

#[derive(Debug)]
pub enum Outcome {
    Ok,
    NoBuilder,
    MissingToolchain(&'static str),
    Failed {
        step: &'static str,
        diagnostics: String,
    },
}

impl Outcome {
    pub fn into_error(self) -> Result<(), DreamError> {
        match self {
            Self::Ok => Ok(()),
            Self::NoBuilder => Err(DreamError::runtime(
                "Dream does not know how to build this target",
            )),
            Self::MissingToolchain(hint) => Err(DreamError::runtime(hint)),
            Self::Failed { step, .. } => Err(DreamError::runtime(format!("{step} failed"))),
        }
    }
}
