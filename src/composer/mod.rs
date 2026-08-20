use crate::error::DreamError;
use crate::tools::Registry;

pub fn run() -> Result<(), DreamError> {
    let _registry = Registry::composer();
    Err(DreamError::new(
        "composition is not implemented yet. Use: dream now <file.foo>",
    ))
}
