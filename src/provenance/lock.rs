use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::DreamError;
use crate::source::paths::{self, rel_path};
use crate::source::Project;
use crate::toolchain::Toolchain;

use super::open::require_store;
use super::store::Store;

pub fn lock(dest: &Path, target: &str, named: &Path) -> Result<(), DreamError> {
    let mut store = require_store(dest, target)?;
    match resolve(&store, dest, named)? {
        Name::Setup(rel) => {
            if !dest.join(&rel).is_file() {
                return Err(DreamError::usage(format!("`{rel}` does not exist")));
            }
            store.lock_file(&rel);
        }
        Name::Unit(unit) => lock_unit(&mut store, dest, &unit, named)?,
    }
    store.save(dest)
}

pub fn unlock(dest: &Path, target: &str, named: &Path) -> Result<(), DreamError> {
    let mut store = require_store(dest, target)?;
    let name = match resolve(&store, dest, named)? {
        Name::Unit(name) | Name::Setup(name) => name,
    };
    if !store.is_locked(&name) {
        return Err(DreamError::usage(format!("`{name}` is not locked")));
    }
    store.clear_lock(&name);
    store.save(dest)
}

pub fn check(store: &Store, dest: &Path, project: &Project) -> Result<(), DreamError> {
    for (unit, state) in &store.units {
        if !state.locked {
            continue;
        }
        if state.source_hash.as_deref() != Some(hash_unit(project, unit)?.as_str()) {
            return Err(DreamError::composer(format!(
                "locked unit `{unit}` source changed; unlock or restore the .foo"
            )));
        }
        missing(dest, &state.artifacts, |rel| {
            format!("locked output `{rel}` for `{unit}` is missing; restore the file, unlock, or --fresh")
        })?;
    }
    missing(dest, &store.locked_setup, |rel| {
        format!("locked setup file `{rel}` is missing; restore the file, unlock, or --fresh")
    })
}

enum Name {
    Unit(String),
    Setup(String),
}

fn resolve(store: &Store, dest: &Path, named: &Path) -> Result<Name, DreamError> {
    if paths::is_foo(named) {
        return Ok(Name::Unit(unit_from_root(store, named)?));
    }
    let rel = dest_name(dest, named);
    if !is_setup(store, &rel) {
        return Err(DreamError::usage(format!("`{rel}` is not a setup file")));
    }
    Ok(Name::Setup(rel))
}

fn lock_unit(store: &mut Store, dest: &Path, unit: &str, named: &Path) -> Result<(), DreamError> {
    let Some(state) = store
        .units
        .get(unit)
        .filter(|state| !state.artifacts.is_empty())
    else {
        return Err(DreamError::usage(format!(
            "`{unit}` has no output files for toolchain `{}`",
            store.toolchain
        )));
    };
    let artifacts = state.artifacts.clone();
    let locked = state.locked;
    let source_hash = state.source_hash.clone();
    missing(dest, &artifacts, |rel| {
        format!(
            "locked output `{rel}` for `{unit}` is missing; restore the file, unlock, or --fresh"
        )
    })?;
    if !named.is_file() {
        return Err(if locked {
            DreamError::composer(format!(
                "locked unit `{unit}` is missing; unlock or restore the .foo"
            ))
        } else {
            DreamError::usage(format!("`{unit}` does not exist"))
        });
    }
    let hash = digest(&fs::read_to_string(named)?);
    if locked {
        return if source_hash.as_deref() == Some(hash.as_str()) {
            Ok(())
        } else {
            Err(DreamError::composer(format!(
                "`{unit}` is locked with a different source; unlock first"
            )))
        };
    }
    store.set_lock(unit, hash);
    Ok(())
}

fn missing(dest: &Path, rels: &[String], err: impl Fn(&str) -> String) -> Result<(), DreamError> {
    rels.iter()
        .find(|rel| !dest.join(rel).is_file())
        .map(|rel| Err(DreamError::composer(err(rel))))
        .unwrap_or(Ok(()))
}

fn dest_name(dest: &Path, named: &Path) -> String {
    rel_path(dest, named).unwrap_or_else(|_| {
        named
            .to_string_lossy()
            .trim_start_matches("./")
            .replace('\\', "/")
    })
}

fn is_setup(store: &Store, rel: &str) -> bool {
    Toolchain::parse(&store.toolchain)
        .ok()
        .and_then(Toolchain::spec)
        .is_some_and(|spec| spec.is_setup(rel))
        || store.project.iter().any(|path| path == rel)
}

fn unit_from_root(store: &Store, source_file: &Path) -> Result<String, DreamError> {
    let root = PathBuf::from(
        store
            .source_root
            .as_deref()
            .ok_or_else(|| DreamError::usage("output has no project root; compose first"))?,
    );
    let file = source_file.canonicalize().unwrap_or_else(|_| {
        if source_file.is_absolute() {
            source_file.to_path_buf()
        } else {
            root.join(source_file)
        }
    });
    rel_path(&root, &file).map_err(|_| {
        DreamError::usage(format!(
            "`{}` is not in the composed project",
            source_file.display()
        ))
    })
}

fn hash_unit(project: &Project, unit: &str) -> Result<String, DreamError> {
    match project.read_foo_file(unit) {
        Ok(read) => Ok(digest(read.source.as_bytes())),
        Err(err) if err.detail().contains("does not exist") => Err(DreamError::composer(format!(
            "locked unit `{unit}` is missing; unlock or restore the .foo"
        ))),
        Err(err) => Err(err),
    }
}

pub(crate) fn source_digest(source: &str) -> String {
    digest(source.as_bytes())
}

fn digest(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Project;
    use std::collections::HashSet;
    use std::fs;

    fn composed(
        entry: &str,
        source: &str,
    ) -> (tempfile::TempDir, tempfile::TempDir, Project, String) {
        let src = tempfile::tempdir().unwrap();
        let file = src.path().join(entry);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, source).unwrap();
        let (project, unit) = Project::from_entry(&file).unwrap();
        let dest = tempfile::tempdir().unwrap();
        fs::create_dir_all(dest.path().join("src")).unwrap();
        fs::write(dest.path().join("src/main.rs"), "fn main() {}").unwrap();
        let mut store = Store::new("cargo");
        store.set_source_root(src.path()).unwrap();
        store.set_artifacts(&unit.rel, HashSet::from(["src/main.rs".into()]));
        store.save(dest.path()).unwrap();
        (src, dest, project, unit.rel)
    }

    #[test]
    fn lock_then_unlock() {
        let (src, dest, _, unit) = composed("main.foo", "print hi");
        let file = src.path().join(&unit);
        lock(dest.path(), "rust", &file).unwrap();
        assert!(Store::load(dest.path()).unwrap().unwrap().is_locked(&unit));
        unlock(dest.path(), "rust", &file).unwrap();
        assert!(!Store::load(dest.path()).unwrap().unwrap().is_locked(&unit));
    }

    #[test]
    fn check_rejects_a_changed_source() {
        let (src, dest, project, unit) = composed("main.foo", "print hi");
        lock(dest.path(), "rust", &src.path().join(&unit)).unwrap();
        fs::write(src.path().join("main.foo"), "print bye").unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        assert!(check(&store, dest.path(), &project)
            .unwrap_err()
            .to_string()
            .contains("source changed"));
    }

    #[test]
    fn lock_names_a_setup_file() {
        let (src, dest, _, unit) = composed("main.foo", "print hi");
        fs::write(dest.path().join("go.mod"), "module x\n").unwrap();
        let mut store = Store::load(dest.path()).unwrap().unwrap();
        store.toolchain = "go".into();
        store.mark_project("go.mod");
        store.save(dest.path()).unwrap();

        lock(dest.path(), "go", &src.path().join(&unit)).unwrap();
        lock(dest.path(), "go", Path::new("go.mod")).unwrap();
        let loaded = Store::load(dest.path()).unwrap().unwrap();
        assert!(loaded.is_locked(&unit) && loaded.is_locked("go.mod"));
        unlock(dest.path(), "go", Path::new("go.mod")).unwrap();
        assert!(!Store::load(dest.path())
            .unwrap()
            .unwrap()
            .is_locked("go.mod"));
    }
}
