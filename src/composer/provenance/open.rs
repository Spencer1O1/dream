use std::fs;
use std::path::Path;

use crate::error::DreamError;

use super::scan::has_user_files;
use super::store::Store;

pub fn open(dest: &Path, target: &str, fresh: bool) -> Result<(Store, bool), DreamError> {
    fs::create_dir_all(dest)?;
    match Store::load(dest)? {
        Some(store) if store.target != target && !fresh => Err(DreamError::usage(format!(
            "output is for target `{}`; pass --fresh to compose `-t {target}`",
            store.target
        ))),
        Some(store) if fresh => {
            store.drop_owned(dest)?;
            Ok((Store::new(target), true))
        }
        Some(store) => Ok((store, false)),
        None if has_user_files(dest)? && !fresh => Err(DreamError::usage(
            "output has files Dream does not own; pass --fresh or use an empty directory",
        )),
        None => Ok((Store::new(target), fresh)),
    }
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
        assert!(open(dest.path(), "rust", true).is_ok());
    }

    #[test]
    fn open_rejects_target_mismatch_unless_fresh() {
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts("main.foo", HashSet::from(["src/main.rs".into()]));
        fs::create_dir_all(dest.path().join("src")).unwrap();
        fs::write(dest.path().join("src/main.rs"), "fn main() {}").unwrap();
        store.save(dest.path()).unwrap();

        let err = open(dest.path(), "go", false).unwrap_err();
        assert!(err.to_string().contains("--fresh"));

        let (fresh_store, is_fresh) = open(dest.path(), "go", true).unwrap();
        assert!(is_fresh);
        assert_eq!(fresh_store.target, "go");
        assert!(!dest.path().join("src/main.rs").exists());
        assert!(!Store::path(dest.path()).exists());
    }
}
