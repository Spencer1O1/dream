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
        Some(store) if !fresh && !accepts_target(&store.toolchain, target) => {
            Err(DreamError::usage(format!(
                "output is for toolchain `{}`; pass --fresh to overwrite `-t {target}`",
                store.toolchain
            )))
        }
        Some(store) if fresh => {
            store.drop_owned(dest)?;
            drop_catalog_project(dest)?;
            // Seed. Compose overwrites with the catalog row after resolve.
            Ok((Store::new(target), true))
        }
        Some(store) => Ok((store, false)),
        None if has_user_files(dest)? && !fresh => Err(DreamError::usage(
            "output has files Dream does not own; pass --fresh to overwrite or use an empty directory",
        )),
        None => {
            if fresh {
                drop_catalog_project(dest)?;
            }
            Ok((Store::new(target), fresh))
        }
    }
}

pub fn require_store(dest: &Path, target: &str) -> Result<Store, DreamError> {
    let Some(store) = Store::load(dest)? else {
        return Err(DreamError::usage(
            "output has no provenance store; compose first",
        ));
    };
    if !accepts_target(&store.toolchain, target) {
        return Err(DreamError::usage(format!(
            "output is for toolchain `{}`; pass `-t {}`",
            store.toolchain, store.toolchain
        )));
    }
    Ok(store)
}

/// A catalog store may reuse a fuzzy `-t`. A non-row store (`monkey_c`) is exact only.
/// The word `unsupported` is not a catalog row and does not fuzzy-match.
pub fn accepts_target(store_toolchain: &str, requested: &str) -> bool {
    store_toolchain == requested
        || (crate::toolchain::spec(store_toolchain).is_some()
            && Toolchain::parse(requested).is_err())
}

/// Already-bound exec for this dest. `None` means resolve (or a first catalog bind).
pub fn existing_bind(store_toolchain: &str, requested: &str, fresh: bool) -> Option<Toolchain> {
    if fresh || !accepts_target(store_toolchain, requested) {
        return None;
    }
    match Toolchain::parse(store_toolchain) {
        Ok(known) if known.spec().is_some() => Some(known),
        _ => Some(Toolchain::Unsupported),
    }
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
        let mut store = Store::new("cargo");
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
        assert_eq!(fresh_store.toolchain, "go");
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
        assert_eq!(opened.toolchain, "cargo");
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

    #[test]
    fn non_row_store_is_exact_only() {
        assert!(accepts_target("monkey_c", "monkey_c"));
        assert!(!accepts_target("monkey_c", "cobol"));
        assert!(!accepts_target("unsupported", "monkey_c"));
        assert!(accepts_target("unsupported", "unsupported"));
        assert!(accepts_target("cargo", "rust"));
        assert!(!accepts_target("cargo", "go"));
    }

    #[test]
    fn existing_bind_reuses_a_row_or_a_matching_target() {
        assert!(existing_bind("cargo", "rust", true).is_none());
        assert_eq!(
            existing_bind("cargo", "rust", false)
                .unwrap()
                .spec()
                .unwrap()
                .name,
            "cargo"
        );
        assert!(matches!(
            existing_bind("monkey_c", "monkey_c", false),
            Some(Toolchain::Unsupported)
        ));
        assert!(existing_bind("monkey_c", "cobol", false).is_none());
        assert!(existing_bind("unsupported", "monkey_c", false).is_none());
        assert!(matches!(
            existing_bind("unsupported", "unsupported", false),
            Some(Toolchain::Unsupported)
        ));
    }
}
