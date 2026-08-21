use std::fs;
use std::path::Path;

use crate::error::DreamError;
use crate::source::paths::{rel_output, resolve_output};

pub fn read_file(dest: &Path, path: &str) -> Result<String, DreamError> {
    let abs = resolve_output(dest, path)?;
    if !abs.is_file() {
        return Err(DreamError::composer(format!(
            "dest file `{path}` does not exist"
        )));
    }
    Ok(fs::read_to_string(abs)?)
}

pub fn write_file(dest: &Path, path: &str, contents: &str) -> Result<String, DreamError> {
    let abs = resolve_output(dest, path)?;
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&abs, contents)?;
    rel_output(dest, &abs)
}

#[derive(Debug)]
pub enum Removed {
    Ok(String),
    Missing(String),
    Directory(String),
}

pub fn remove_file(dest: &Path, path: &str) -> Result<Removed, DreamError> {
    let abs = resolve_output(dest, path)?;
    let rel = rel_output(dest, &abs)?;
    if abs.is_dir() {
        return Ok(Removed::Directory(rel));
    }
    if !abs.exists() {
        return Ok(Removed::Missing(rel));
    }
    fs::remove_file(&abs)?;
    prune_empty_parents(dest, abs.parent())?;
    Ok(Removed::Ok(rel))
}

/// File or directory. Dest-bounded. Missing is fine.
pub fn remove_dest(dest: &Path, path: &str) -> Result<(), DreamError> {
    let abs = resolve_output(dest, path)?;
    if !abs.exists() {
        return Ok(());
    }
    if abs.is_dir() {
        fs::remove_dir_all(abs)?;
        return Ok(());
    }
    fs::remove_file(&abs)?;
    prune_empty_parents(dest, abs.parent())?;
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
        assert!(err.to_string().contains("dest write escapes -o"));
        assert!(!dest.path().join("outside.rs").exists());
    }

    #[test]
    fn remove_file_deletes_and_prunes_empty_parents() {
        let dest = tempfile::tempdir().unwrap();
        write_file(dest.path(), "src/oops.rs", "nope").unwrap();
        write_file(dest.path(), "keep.txt", "keep").unwrap();
        assert!(matches!(
            remove_file(dest.path(), "src/oops.rs").unwrap(),
            Removed::Ok(rel) if rel == "src/oops.rs"
        ));
        assert!(!dest.path().join("src/oops.rs").exists());
        assert!(!dest.path().join("src").exists());
        assert_eq!(
            fs::read_to_string(dest.path().join("keep.txt")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn remove_file_missing_directory_and_escape() {
        let dest = tempfile::tempdir().unwrap();
        write_file(dest.path(), "keep.txt", "keep").unwrap();
        fs::create_dir(dest.path().join("lib")).unwrap();

        assert!(matches!(
            remove_file(dest.path(), "gone.rs").unwrap(),
            Removed::Missing(rel) if rel == "gone.rs"
        ));

        assert!(matches!(
            remove_file(dest.path(), "lib").unwrap(),
            Removed::Directory(rel) if rel == "lib"
        ));
        assert!(dest.path().join("lib").is_dir());

        let escape = remove_file(dest.path(), "../secret").unwrap_err();
        assert!(escape.to_string().contains("dest write escapes -o"));
    }

    #[test]
    fn remove_dest_deletes_a_directory_tree() {
        let dest = tempfile::tempdir().unwrap();
        fs::create_dir_all(dest.path().join("target/debug")).unwrap();
        fs::write(dest.path().join("target/debug/x"), "bin").unwrap();
        fs::write(dest.path().join("keep.txt"), "keep").unwrap();
        remove_dest(dest.path(), "target").unwrap();
        assert!(!dest.path().join("target").exists());
        assert_eq!(
            fs::read_to_string(dest.path().join("keep.txt")).unwrap(),
            "keep"
        );
        remove_dest(dest.path(), "missing.lock").unwrap();
        let escape = remove_dest(dest.path(), "../secret").unwrap_err();
        assert!(escape.to_string().contains("dest write escapes -o"));
    }
}
