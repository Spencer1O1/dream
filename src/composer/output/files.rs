use std::fs;
use std::path::Path;

use crate::error::DreamError;
use crate::source::paths::{rel_output, resolve_output};

pub fn write_file(staging: &Path, path: &str, contents: &str) -> Result<String, DreamError> {
    let dest = resolve_output(staging, path)?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&dest, contents)?;
    rel_output(staging, &dest)
}

pub fn remove_file(staging: &Path, path: &str) -> Result<String, DreamError> {
    let dest = resolve_output(staging, path)?;
    if dest.is_dir() {
        return Err(DreamError::runtime(format!(
            "output path `{path}` is a directory"
        )));
    }
    if !dest.exists() {
        return Err(DreamError::runtime(format!(
            "output file `{path}` does not exist"
        )));
    }
    fs::remove_file(&dest)?;
    prune_empty_parents(staging, dest.parent())?;
    rel_output(staging, &dest)
}

pub fn tree_has_files(dir: &Path) -> Result<bool, DreamError> {
    if !dir.exists() {
        return Ok(false);
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            if tree_has_files(&path)? {
                return Ok(true);
            }
        } else {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn require_files(dir: &Path) -> Result<(), DreamError> {
    if !tree_has_files(dir)? {
        return Err(DreamError::runtime("composition produced no files"));
    }
    Ok(())
}

fn prune_empty_parents(root: &Path, mut dir: Option<&Path>) -> Result<(), DreamError> {
    while let Some(current) = dir {
        if current == root {
            break;
        }
        if fs::remove_dir(current).is_err() {
            break;
        }
        dir = current.parent();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn write_file_stays_in_staging() {
        let staging = tempfile::tempdir().unwrap();
        let rel = write_file(staging.path(), "src/main.rs", "fn main() {}").unwrap();
        assert_eq!(rel, "src/main.rs");
        assert_eq!(
            fs::read_to_string(staging.path().join("src/main.rs")).unwrap(),
            "fn main() {}"
        );
    }

    #[test]
    fn write_file_rejects_escape() {
        let staging = tempfile::tempdir().unwrap();
        let err = write_file(staging.path(), "../outside.rs", "no").unwrap_err();
        assert!(err.to_string().contains("output write escapes -o"));
        assert!(!tree_has_files(staging.path()).unwrap());
    }

    #[test]
    fn remove_file_deletes_and_prunes_empty_parents() {
        let staging = tempfile::tempdir().unwrap();
        write_file(staging.path(), "src/oops.rs", "nope").unwrap();
        write_file(staging.path(), "keep.txt", "keep").unwrap();
        let rel = remove_file(staging.path(), "src/oops.rs").unwrap();
        assert_eq!(rel, "src/oops.rs");
        assert!(!staging.path().join("src/oops.rs").exists());
        assert!(!staging.path().join("src").exists());
        assert_eq!(
            fs::read_to_string(staging.path().join("keep.txt")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn remove_file_rejects_missing_directory_and_escape() {
        let staging = tempfile::tempdir().unwrap();
        write_file(staging.path(), "keep.txt", "keep").unwrap();
        fs::create_dir(staging.path().join("lib")).unwrap();

        let missing = remove_file(staging.path(), "gone.rs").unwrap_err();
        assert!(missing.to_string().contains("does not exist"));

        let dir = remove_file(staging.path(), "lib").unwrap_err();
        assert!(dir.to_string().contains("is a directory"));
        assert!(staging.path().join("lib").is_dir());

        let escape = remove_file(staging.path(), "../secret").unwrap_err();
        assert!(escape.to_string().contains("output write escapes -o"));
    }

    #[test]
    fn no_files_is_an_error() {
        let staging = tempfile::tempdir().unwrap();
        let err = require_files(staging.path()).unwrap_err();
        assert!(err.to_string().contains("produced no files"));
    }

    #[test]
    fn empty_dirs_are_not_files() {
        let staging = tempfile::tempdir().unwrap();
        fs::create_dir(staging.path().join("empty")).unwrap();
        assert!(!tree_has_files(staging.path()).unwrap());
    }
}
