use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::DreamError;
use crate::source::paths::rel_path;
use crate::source::Project;

use super::store::Store;

pub fn lock(dest: &Path, target: &str, source_file: &Path) -> Result<(), DreamError> {
    let mut store = require_store(dest, target)?;
    let unit = unit_from_root(&store, source_file)?;
    let Some(state) = store
        .units
        .get(&unit)
        .filter(|state| !state.artifacts.is_empty())
    else {
        return Err(DreamError::usage(format!(
            "`{unit}` has no artifacts for target `{target}`"
        )));
    };
    let artifacts = state.artifacts.clone();
    let locked = state.locked;
    let source_hash = state.source_hash.clone();
    require_artifacts(dest, &unit, &artifacts)?;
    if !source_file.is_file() {
        return Err(if locked {
            DreamError::composer(format!(
                "locked unit `{unit}` is missing; unlock or restore the .foo"
            ))
        } else {
            DreamError::usage(format!("`{unit}` does not exist"))
        });
    }
    let hash = hash_file(source_file)?;
    if locked {
        if source_hash.as_deref() != Some(hash.as_str()) {
            return Err(DreamError::composer(format!(
                "`{unit}` is locked with a different source; unlock first"
            )));
        }
        return Ok(());
    }
    store.set_lock(&unit, hash);
    store.save(dest)?;
    Ok(())
}

pub fn unlock(dest: &Path, target: &str, source_file: &Path) -> Result<(), DreamError> {
    let mut store = require_store(dest, target)?;
    let unit = unit_from_root(&store, source_file)?;
    if !store.is_locked(&unit) {
        return Err(DreamError::usage(format!("`{unit}` is not locked")));
    }
    store.clear_lock(&unit);
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
            return Err(DreamError::composer(format!(
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
            return Err(DreamError::composer(format!(
                "locked artifact `{rel}` for `{unit}` is missing; restore the file, unlock, or --fresh"
            )));
        }
    }
    Ok(())
}

fn unit_from_root(store: &Store, source_file: &Path) -> Result<String, DreamError> {
    let root = store
        .source_root
        .as_deref()
        .ok_or_else(|| DreamError::usage("output has no project root; compose first"))?;
    let root = PathBuf::from(root);
    let file = match source_file.canonicalize() {
        Ok(path) => path,
        Err(_) if source_file.is_absolute() => source_file.to_path_buf(),
        Err(_) => root.join(source_file),
    };
    rel_path(&root, &file).map_err(|_| {
        DreamError::usage(format!(
            "`{}` is not in the composed project",
            source_file.display()
        ))
    })
}

fn hash_unit(project: &Project, unit: &str) -> Result<String, DreamError> {
    match project.read_source_file(unit) {
        Ok(read) => Ok(hex_sha256(read.source.as_bytes())),
        Err(err) if err.detail().contains("does not exist") => Err(DreamError::composer(format!(
            "locked unit `{unit}` is missing; unlock or restore the .foo"
        ))),
        Err(err) => Err(err),
    }
}

fn hash_file(path: &Path) -> Result<String, DreamError> {
    let source = fs::read_to_string(path)?;
    Ok(hex_sha256(source.as_bytes()))
}

pub(crate) fn source_digest(source: &str) -> String {
    hex_sha256(source.as_bytes())
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
        let file = dir.path().join(entry);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&file, source).unwrap();
        let (project, unit) = Project::from_entry(&file).unwrap();
        (dir, project, unit.rel)
    }

    fn dest_with(root: &Path, unit: &str, rel: &str, contents: &str) -> (tempfile::TempDir, Store) {
        let dest = tempfile::tempdir().unwrap();
        if let Some(parent) = dest.path().join(rel).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(dest.path().join(rel), contents).unwrap();
        let mut store = Store::new("rust");
        store.set_source_root(root).unwrap();
        store.set_artifacts(unit, HashSet::from([rel.to_string()]));
        store.save(dest.path()).unwrap();
        (dest, store)
    }

    #[test]
    fn lock_then_unlock() {
        let (src, _project, unit) = project_with("main.foo", "print hi");
        let file = src.path().join(&unit);
        let (dest, _) = dest_with(src.path(), &unit, "src/main.rs", "fn main() {}");
        lock(dest.path(), "rust", &file).unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        assert!(store.is_locked(&unit));
        assert!(store.units[&unit].source_hash.is_some());
        unlock(dest.path(), "rust", &file).unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        assert!(!store.is_locked(&unit));
    }

    #[test]
    fn check_errors_when_source_changes() {
        let (src, project, unit) = project_with("main.foo", "print hi");
        let (dest, _) = dest_with(src.path(), &unit, "src/main.rs", "fn main() {}");
        lock(dest.path(), "rust", &src.path().join(&unit)).unwrap();
        fs::write(src.path().join("main.foo"), "print bye").unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        let err = check(&store, dest.path(), &project).unwrap_err();
        assert!(err.to_string().starts_with("ComposerError:"));
        assert!(err.to_string().contains("source changed"));
    }

    #[test]
    fn check_errors_when_locked_source_is_missing() {
        let (src, project, unit) = project_with("main.foo", "print hi");
        let (dest, _) = dest_with(src.path(), &unit, "src/main.rs", "fn main() {}");
        lock(dest.path(), "rust", &src.path().join(&unit)).unwrap();
        fs::remove_file(src.path().join("main.foo")).unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        let err = check(&store, dest.path(), &project).unwrap_err();
        assert!(err.to_string().starts_with("ComposerError:"));
        assert!(err.to_string().contains("is missing"));
        assert!(!err.to_string().starts_with("RuntimeError:"));
    }

    #[test]
    fn lock_errors_when_locked_source_is_missing() {
        let (src, _project, unit) = project_with("main.foo", "print hi");
        let file = src.path().join(&unit);
        let (dest, _) = dest_with(src.path(), &unit, "src/main.rs", "fn main() {}");
        lock(dest.path(), "rust", &file).unwrap();
        fs::remove_file(&file).unwrap();
        let err = lock(dest.path(), "rust", &file).unwrap_err();
        assert!(err.to_string().starts_with("ComposerError:"));
        assert!(err.to_string().contains("is missing"));
    }

    #[test]
    fn check_errors_when_an_artifact_is_missing() {
        let (src, project, unit) = project_with("main.foo", "print hi");
        let (dest, _) = dest_with(src.path(), &unit, "src/main.rs", "fn main() {}");
        lock(dest.path(), "rust", &src.path().join(&unit)).unwrap();
        fs::remove_file(dest.path().join("src/main.rs")).unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        let err = check(&store, dest.path(), &project).unwrap_err();
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn lock_same_hash_is_ok_and_different_hash_is_not() {
        let (src, _project, unit) = project_with("main.foo", "print hi");
        let file = src.path().join(&unit);
        let (dest, _) = dest_with(src.path(), &unit, "src/main.rs", "fn main() {}");
        lock(dest.path(), "rust", &file).unwrap();
        lock(dest.path(), "rust", &file).unwrap();
        fs::write(src.path().join("main.foo"), "print bye").unwrap();
        let err = lock(dest.path(), "rust", &file).unwrap_err();
        assert!(err.to_string().starts_with("ComposerError:"));
        assert!(err.to_string().contains("unlock first"));
    }

    #[test]
    fn unlock_of_unlocked_is_an_error() {
        let (src, _project, unit) = project_with("main.foo", "print hi");
        let (dest, _) = dest_with(src.path(), &unit, "src/main.rs", "fn main() {}");
        let err = unlock(dest.path(), "rust", &src.path().join(&unit)).unwrap_err();
        assert!(err.to_string().contains("not locked"));
    }

    #[test]
    fn lock_requires_artifacts() {
        let (src, _project, unit) = project_with("main.foo", "print hi");
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("rust");
        store.set_source_root(src.path()).unwrap();
        store.save(dest.path()).unwrap();
        let err = lock(dest.path(), "rust", &src.path().join(&unit)).unwrap_err();
        assert!(err.to_string().contains("no artifacts"));
    }

    #[test]
    fn lock_requires_a_composed_project_root() {
        let (src, _project, unit) = project_with("main.foo", "print hi");
        let dest = tempfile::tempdir().unwrap();
        Store::new("rust").save(dest.path()).unwrap();
        let err = lock(dest.path(), "rust", &src.path().join(&unit)).unwrap_err();
        assert!(err.to_string().contains("project root"));
    }

    #[test]
    fn check_allows_a_hand_edited_artifact() {
        let (src, project, unit) = project_with("main.foo", "print hi");
        let (dest, _) = dest_with(src.path(), &unit, "src/main.rs", "fn main() {}");
        lock(dest.path(), "rust", &src.path().join(&unit)).unwrap();
        fs::write(dest.path().join("src/main.rs"), "fn main() { /* hand */ }").unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        check(&store, dest.path(), &project).unwrap();
    }

    #[test]
    fn lock_matches_a_nested_store_key() {
        let src = tempfile::tempdir().unwrap();
        fs::create_dir_all(src.path().join("users")).unwrap();
        fs::write(src.path().join("main.foo"), "entry").unwrap();
        fs::write(src.path().join("users/active.foo"), "active").unwrap();
        let (dest, _) = dest_with(
            src.path(),
            "users/active.foo",
            "src/active.rs",
            "fn active() {}",
        );
        lock(dest.path(), "rust", &src.path().join("users/active.foo")).unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        assert!(store.is_locked("users/active.foo"));
        assert!(!store.units.contains_key("active.foo"));
        unlock(dest.path(), "rust", &src.path().join("users/active.foo")).unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        assert!(!store.is_locked("users/active.foo"));
    }

    #[test]
    fn lock_key_is_the_path_from_the_project_root() {
        let src = tempfile::tempdir().unwrap();
        fs::create_dir_all(src.path().join("users")).unwrap();
        fs::write(src.path().join("active.foo"), "root").unwrap();
        fs::write(src.path().join("users/active.foo"), "nested").unwrap();
        let dest = tempfile::tempdir().unwrap();
        fs::create_dir_all(dest.path().join("src")).unwrap();
        fs::write(dest.path().join("src/root.rs"), "fn root() {}").unwrap();
        fs::write(dest.path().join("src/nested.rs"), "fn nested() {}").unwrap();
        let mut store = Store::new("rust");
        store.set_source_root(src.path()).unwrap();
        store.set_artifacts("active.foo", HashSet::from(["src/root.rs".into()]));
        store.set_artifacts("users/active.foo", HashSet::from(["src/nested.rs".into()]));
        store.save(dest.path()).unwrap();
        lock(dest.path(), "rust", &src.path().join("users/active.foo")).unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        assert!(store.is_locked("users/active.foo"));
        assert!(!store.is_locked("active.foo"));
    }
}
