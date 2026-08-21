use std::fs;
use std::path::Path;

use crate::error::DreamError;
use crate::output;
use crate::toolchain::{Toolchain, CATALOG};

use super::scan::has_user_files;
use super::store::Store;

pub fn open(dest: &Path, target: &str, fresh: bool) -> Result<(Store, bool), DreamError> {
    fs::create_dir_all(dest)?;
    match Store::load(dest)? {
        Some(store) if !fresh && !accepts_target(&store.target, target) => {
            Err(DreamError::usage(format!(
                "dest is for target `{}`; pass --fresh to overwrite `-t {target}`",
                store.target
            )))
        }
        Some(store) if fresh => {
            store.drop_owned(dest)?;
            drop_catalog_project(dest)?;
            Ok((Store::new(target), true))
        }
        Some(store) => Ok((store, false)),
        None if has_user_files(dest)? && !fresh => Err(DreamError::usage(
            "dest has files Dream does not own; pass --fresh to overwrite or use an empty directory",
        )),
        None => {
            if fresh {
                drop_catalog_project(dest)?;
            }
            Ok((Store::new(target), fresh))
        }
    }
}

/// Exact catalog name must match. A fuzzy `-t` may reuse a catalog store.
pub fn accepts_target(store_target: &str, requested: &str) -> bool {
    store_target == requested
        || (Toolchain::parse(requested).is_err() && Toolchain::parse(store_target).is_ok())
}

fn drop_catalog_project(dest: &Path) -> Result<(), DreamError> {
    for spec in CATALOG {
        for path in spec.owned_dest() {
            output::remove_dest(dest, &path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;

    #[test]
    fn open_errors_when_files_exist_without_a_store() {
        let dest = tempfile::tempdir().unwrap();
        fs::write(dest.path().join("README.md"), "hi").unwrap();
        let err = open(dest.path(), "rust", false).unwrap_err();
        assert!(err.to_string().contains("--fresh"));
        assert!(open(dest.path(), "rust", true).is_ok());
    }

    #[test]
    fn leftover_target_without_a_store_is_occupied() {
        let dest = tempfile::tempdir().unwrap();
        fs::create_dir_all(dest.path().join("target/debug")).unwrap();
        fs::write(dest.path().join("target/debug/x"), "bin").unwrap();
        let err = open(dest.path(), "rust", false).unwrap_err();
        assert!(err.to_string().contains("--fresh"));
        open(dest.path(), "rust", true).unwrap();
        assert!(!dest.path().join("target").exists());
    }

    #[test]
    fn open_rejects_target_mismatch_unless_fresh() {
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts("main.foo", HashSet::from(["src/main.rs".into()]));
        store.mark_project("Cargo.toml");
        fs::create_dir_all(dest.path().join("src")).unwrap();
        fs::write(dest.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(dest.path().join("Cargo.toml"), "[package]\n").unwrap();
        fs::write(dest.path().join("Cargo.lock"), "# lock\n").unwrap();
        fs::write(dest.path().join("README.md"), "keep").unwrap();
        store.save(dest.path()).unwrap();

        let err = open(dest.path(), "go", false).unwrap_err();
        assert!(err.to_string().contains("--fresh"));

        let (fresh_store, is_fresh) = open(dest.path(), "go", true).unwrap();
        assert!(is_fresh);
        assert_eq!(fresh_store.target, "go");
        assert!(!dest.path().join("src/main.rs").exists());
        assert!(!dest.path().join("Cargo.toml").exists());
        assert!(!dest.path().join("Cargo.lock").exists());
        assert_eq!(
            fs::read_to_string(dest.path().join("README.md")).unwrap(),
            "keep"
        );
        assert!(!Store::path(dest.path()).exists());
    }

    #[test]
    fn fuzzy_requested_reuses_a_catalog_store() {
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("cargo");
        store.mark_project("Cargo.toml");
        store.save(dest.path()).unwrap();
        let (opened, fresh) = open(dest.path(), "rust", false).unwrap();
        assert!(!fresh);
        assert_eq!(opened.target, "cargo");
        let err = open(dest.path(), "go", false).unwrap_err();
        assert!(err.to_string().contains("`cargo`"));
    }

    #[test]
    fn fresh_drops_a_foreign_manifest_even_if_unlisted() {
        let dest = tempfile::tempdir().unwrap();
        fs::write(dest.path().join("Cargo.toml"), "[package]\n").unwrap();
        fs::write(dest.path().join("README.md"), "keep").unwrap();
        open(dest.path(), "python", true).unwrap();
        assert!(!dest.path().join("Cargo.toml").exists());
        assert_eq!(
            fs::read_to_string(dest.path().join("README.md")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn dest_root_binary_is_a_user_file() {
        let dest = tempfile::tempdir().unwrap();
        fs::write(dest.path().join("hey-you"), "binary").unwrap();
        fs::write(dest.path().join("README.md"), "keep").unwrap();
        open(dest.path(), "go", true).unwrap();
        assert_eq!(
            fs::read_to_string(dest.path().join("hey-you")).unwrap(),
            "binary"
        );
        assert_eq!(
            fs::read_to_string(dest.path().join("README.md")).unwrap(),
            "keep"
        );
    }
}
