use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::error::DreamError;

use super::store::Store;

pub fn require_composed(
    artifacts: &HashMap<String, HashSet<String>>,
    store: &Store,
) -> Result<(), DreamError> {
    if artifacts.values().any(|paths| !paths.is_empty()) {
        return Ok(());
    }
    if store
        .units
        .values()
        .any(|state| !state.artifacts.is_empty())
    {
        return Ok(());
    }
    Err(DreamError::composer("composition produced no files"))
}

pub fn require_source_root(store: &Store, root: &Path) -> Result<(), DreamError> {
    let Some(prev) = store.source_root.as_deref() else {
        return Ok(());
    };
    let root = root.canonicalize()?;
    if prev != root.to_string_lossy().as_ref() {
        return Err(DreamError::usage(
            "dest is for another Dream project; pass --fresh to overwrite",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::Store;

    #[test]
    fn missing_source_root_is_ok() {
        let store = Store::new("rust");
        let dir = tempfile::tempdir().unwrap();
        require_source_root(&store, dir.path()).unwrap();
    }

    #[test]
    fn same_root_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::new("rust");
        store.set_source_root(dir.path()).unwrap();
        require_source_root(&store, dir.path()).unwrap();
    }

    #[test]
    fn other_root_is_a_usage_error() {
        let dest_project = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let mut store = Store::new("rust");
        store.set_source_root(dest_project.path()).unwrap();
        let err = require_source_root(&store, other.path()).unwrap_err();
        assert!(err.to_string().starts_with("UsageError:"));
        assert!(err.to_string().contains("--fresh"));
    }

    #[test]
    fn composed_means_this_session_or_the_store_has_unit_files() {
        let empty_store = Store::new("cargo");
        let empty = HashMap::new();
        let err = require_composed(&empty, &empty_store).unwrap_err();
        assert!(err.to_string().contains("produced no files"));

        let leftovers_only = HashMap::from([("old.foo".into(), HashSet::new())]);
        assert!(require_composed(&leftovers_only, &empty_store).is_err());

        let wrote = HashMap::from([("hey-you.foo".into(), HashSet::from(["main.rs".into()]))]);
        require_composed(&wrote, &empty_store).unwrap();

        let mut prior = Store::new("cargo");
        prior.set_artifacts("main.foo", HashSet::from(["src/main.rs".into()]));
        require_composed(&empty, &prior).unwrap();
    }
}
