use std::fs;
use std::path::Path;

use crate::error::DreamError;
use crate::source::paths::{rel_output, resolve_output};

pub fn write_file(dest: &Path, path: &str, contents: &str) -> Result<String, DreamError> {
    let abs = resolve_output(dest, path)?;
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&abs, contents)?;
    rel_output(dest, &abs)
}

pub fn remove_file(dest: &Path, path: &str) -> Result<String, DreamError> {
    let abs = resolve_output(dest, path)?;
    if abs.is_dir() {
        return Err(DreamError::composer(format!(
            "output path `{path}` is a directory"
        )));
    }
    if !abs.exists() {
        return Err(DreamError::composer(format!(
            "output file `{path}` does not exist"
        )));
    }
    fs::remove_file(&abs)?;
    prune_empty_parents(dest, abs.parent())?;
    rel_output(dest, &abs)
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
    fn write_file_stays_in_dest() {
        let dest = tempfile::tempdir().unwrap();
        let rel = write_file(dest.path(), "src/main.rs", "fn main() {}").unwrap();
        assert_eq!(rel, "src/main.rs");
        assert_eq!(
            fs::read_to_string(dest.path().join("src/main.rs")).unwrap(),
            "fn main() {}"
        );
    }

    #[test]
    fn write_file_rejects_escape() {
        let dest = tempfile::tempdir().unwrap();
        let err = write_file(dest.path(), "../outside.rs", "no").unwrap_err();
        assert!(err.to_string().contains("output write escapes -o"));
        assert!(!dest.path().join("outside.rs").exists());
    }

    #[test]
    fn remove_file_deletes_and_prunes_empty_parents() {
        let dest = tempfile::tempdir().unwrap();
        write_file(dest.path(), "src/oops.rs", "nope").unwrap();
        write_file(dest.path(), "keep.txt", "keep").unwrap();
        let rel = remove_file(dest.path(), "src/oops.rs").unwrap();
        assert_eq!(rel, "src/oops.rs");
        assert!(!dest.path().join("src/oops.rs").exists());
        assert!(!dest.path().join("src").exists());
        assert_eq!(
            fs::read_to_string(dest.path().join("keep.txt")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn remove_file_rejects_missing_directory_and_escape() {
        let dest = tempfile::tempdir().unwrap();
        write_file(dest.path(), "keep.txt", "keep").unwrap();
        fs::create_dir(dest.path().join("lib")).unwrap();

        let missing = remove_file(dest.path(), "gone.rs").unwrap_err();
        assert!(missing.to_string().contains("does not exist"));

        let dir = remove_file(dest.path(), "lib").unwrap_err();
        assert!(dir.to_string().contains("is a directory"));
        assert!(dest.path().join("lib").is_dir());

        let escape = remove_file(dest.path(), "../secret").unwrap_err();
        assert!(escape.to_string().contains("output write escapes -o"));
    }
}
