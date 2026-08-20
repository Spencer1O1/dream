use std::fmt::Write as _;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::DreamError;
use crate::source::Project;

use super::store::Store;

pub fn lock(dest: &Path, target: &str, project: &Project, unit: &str) -> Result<(), DreamError> {
    let mut store = require_store(dest, target)?;
    let state = store.units.get(unit).ok_or_else(|| {
        DreamError::usage(format!("`{unit}` has no artifacts for target `{target}`"))
    })?;
    if state.artifacts.is_empty() {
        return Err(DreamError::usage(format!(
            "`{unit}` has no artifacts for target `{target}`"
        )));
    }
    require_artifacts(dest, unit, &state.artifacts)?;
    let hash = hash_unit(project, unit)?;
    if state.locked {
        if state.source_hash.as_deref() != Some(hash.as_str()) {
            return Err(DreamError::usage(format!(
                "`{unit}` is locked with a different source; unlock first"
            )));
        }
        return Ok(());
    }
    store.set_lock(unit, hash);
    store.save(dest)?;
    Ok(())
}

pub fn unlock(dest: &Path, target: &str, unit: &str) -> Result<(), DreamError> {
    let mut store = require_store(dest, target)?;
    if !store.is_locked(unit) {
        return Err(DreamError::usage(format!("`{unit}` is not locked")));
    }
    store.clear_lock(unit);
    store.save(dest)?;
    Ok(())
}

pub fn check(store: &Store, dest: &Path, project: &Project) -> Result<(), DreamError> {
    for (unit, state) in &store.units {
        if !state.locked {
            continue;
        }
        let hash = hash_unit(project, unit)?;
        if state.source_hash.as_deref() != Some(hash.as_str()) {
            return Err(DreamError::runtime(format!(
                "locked unit `{unit}` source changed; unlock or restore the .foo"
            )));
        }
        require_artifacts(dest, unit, &state.artifacts)?;
    }
    Ok(())
}

fn require_store(dest: &Path, target: &str) -> Result<Store, DreamError> {
    let Some(store) = Store::load(dest)? else {
        return Err(DreamError::usage(
            "output has no provenance store; compose first",
        ));
    };
    if store.target != target {
        return Err(DreamError::usage(format!(
            "output is for target `{}`; pass `-t {}` or --fresh to compose",
            store.target, store.target
        )));
    }
    Ok(store)
}

fn require_artifacts(dest: &Path, unit: &str, artifacts: &[String]) -> Result<(), DreamError> {
    for rel in artifacts {
        let path = dest.join(rel);
        if !path.is_file() {
            return Err(DreamError::runtime(format!(
                "locked artifact `{rel}` for `{unit}` is missing; restore the file, unlock, or --fresh"
            )));
        }
    }
    Ok(())
}

fn hash_unit(project: &Project, unit: &str) -> Result<String, DreamError> {
    let source = project.read_source_file(unit)?.source;
    Ok(hex_sha256(source.as_bytes()))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Project;
    use std::collections::HashSet;
    use std::fs;

    fn project_with(entry: &str, source: &str) -> (tempfile::TempDir, Project, String) {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(entry), source).unwrap();
        let (project, unit) = Project::from_entry(&dir.path().join(entry)).unwrap();
        (dir, project, unit.rel)
    }

    fn dest_with(unit: &str, rel: &str, contents: &str) -> (tempfile::TempDir, Store) {
        let dest = tempfile::tempdir().unwrap();
        if let Some(parent) = dest.path().join(rel).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(dest.path().join(rel), contents).unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts(unit, HashSet::from([rel.to_string()]));
        store.save(dest.path()).unwrap();
        (dest, store)
    }

    #[test]
    fn lock_then_unlock() {
        let (_src, project, unit) = project_with("main.foo", "print hi");
        let (dest, _) = dest_with(&unit, "src/main.rs", "fn main() {}");
        lock(dest.path(), "rust", &project, &unit).unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        assert!(store.is_locked(&unit));
        assert!(store.units[&unit].source_hash.is_some());
        unlock(dest.path(), "rust", &unit).unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        assert!(!store.is_locked(&unit));
    }

    #[test]
    fn check_errors_when_source_changes() {
        let (src, project, unit) = project_with("main.foo", "print hi");
        let (dest, _) = dest_with(&unit, "src/main.rs", "fn main() {}");
        lock(dest.path(), "rust", &project, &unit).unwrap();
        fs::write(src.path().join("main.foo"), "print bye").unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        let err = check(&store, dest.path(), &project).unwrap_err();
        assert!(err.to_string().contains("source changed"));
    }

    #[test]
    fn check_errors_when_an_artifact_is_missing() {
        let (_src, project, unit) = project_with("main.foo", "print hi");
        let (dest, _) = dest_with(&unit, "src/main.rs", "fn main() {}");
        lock(dest.path(), "rust", &project, &unit).unwrap();
        fs::remove_file(dest.path().join("src/main.rs")).unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        let err = check(&store, dest.path(), &project).unwrap_err();
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn lock_same_hash_is_ok_and_different_hash_is_not() {
        let (src, project, unit) = project_with("main.foo", "print hi");
        let (dest, _) = dest_with(&unit, "src/main.rs", "fn main() {}");
        lock(dest.path(), "rust", &project, &unit).unwrap();
        lock(dest.path(), "rust", &project, &unit).unwrap();
        fs::write(src.path().join("main.foo"), "print bye").unwrap();
        let err = lock(dest.path(), "rust", &project, &unit).unwrap_err();
        assert!(err.to_string().contains("unlock first"));
    }

    #[test]
    fn unlock_of_unlocked_is_an_error() {
        let (_src, _project, unit) = project_with("main.foo", "print hi");
        let (dest, _) = dest_with(&unit, "src/main.rs", "fn main() {}");
        let err = unlock(dest.path(), "rust", &unit).unwrap_err();
        assert!(err.to_string().contains("not locked"));
    }

    #[test]
    fn lock_requires_artifacts() {
        let (_src, project, unit) = project_with("main.foo", "print hi");
        let dest = tempfile::tempdir().unwrap();
        Store::new("rust").save(dest.path()).unwrap();
        let err = lock(dest.path(), "rust", &project, &unit).unwrap_err();
        assert!(err.to_string().contains("no artifacts"));
    }

    #[test]
    fn check_allows_a_hand_edited_artifact() {
        let (_src, project, unit) = project_with("main.foo", "print hi");
        let (dest, _) = dest_with(&unit, "src/main.rs", "fn main() {}");
        lock(dest.path(), "rust", &project, &unit).unwrap();
        fs::write(dest.path().join("src/main.rs"), "fn main() { /* hand */ }").unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        check(&store, dest.path(), &project).unwrap();
    }
}
