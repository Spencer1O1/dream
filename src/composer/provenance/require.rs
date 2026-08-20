use crate::error::DreamError;

use super::store::Store;

pub fn require_composed(store: &Store) -> Result<(), DreamError> {
    if store.has_artifacts() {
        Ok(())
    } else {
        Err(DreamError::runtime("composition produced no files"))
    }
}
