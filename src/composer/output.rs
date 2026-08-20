use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::DreamError;
use crate::source::paths::{rel_output, resolve_output};

pub fn resolve_output_dir(project_root: &Path, output: &Path) -> Result<PathBuf, DreamError> {
    if output.as_os_str().is_empty() {
        return Err(DreamError::usage("compose requires -o <dir>"));
    }

    let output = if is_cwd(output) {
        std::env::current_dir()?
    } else if output.exists() {
        if output.is_file() {
            return Err(DreamError::usage(format!(
                "output `{}` is a file, not a directory",
                output.display()
            )));
        }
        output.canonicalize()?
    } else {
        let parent = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let name = output
            .file_name()
            .ok_or_else(|| DreamError::usage("output directory has no name"))?;
        parent.canonicalize()?.join(name)
    };

    if output == project_root || project_root.starts_with(&output) {
        return Err(DreamError::usage(
            "output directory would replace the Dream project",
        ));
    }

    Ok(output)
}

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

pub fn replace_output(output: &Path, staging: &Path) -> Result<(), DreamError> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let backup = if output.exists() {
        let backup = sibling_backup(output)?;
        fs::rename(output, &backup)?;
        Some(backup)
    } else {
        None
    };

    match persist_tree(staging, output) {
        Ok(()) => {
            if let Some(backup) = backup {
                let _ = fs::remove_dir_all(backup);
            }
            Ok(())
        }
        Err(err) => {
            if let Some(backup) = backup {
                let _ = fs::remove_dir_all(output);
                let _ = fs::rename(backup, output);
            }
            Err(err)
        }
    }
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

fn is_cwd(path: &Path) -> bool {
    path == Path::new(".") || path == Path::new("./")
}

fn sibling_backup(output: &Path) -> Result<PathBuf, DreamError> {
    let parent = output
        .parent()
        .ok_or_else(|| DreamError::runtime("output directory has no parent"))?;
    let name = output
        .file_name()
        .ok_or_else(|| DreamError::runtime("output directory has no name"))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    Ok(parent.join(format!(".{}.dream-old-{stamp}", name.to_string_lossy())))
}

fn persist_tree(from: &Path, to: &Path) -> Result<(), DreamError> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    copy_tree(from, to)?;
    fs::remove_dir_all(from)?;
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), DreamError> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let src = entry.path();
        let dest = to.join(entry.file_name());
        if src.is_dir() {
            copy_tree(&src, &dest)?;
        } else {
            fs::copy(&src, &dest)?;
        }
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
    fn replace_output_replaces_existing_tree() {
        let parent = tempfile::tempdir().unwrap();
        let dest = parent.path().join("out");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("old.txt"), "old").unwrap();

        let staging = parent.path().join("stage");
        fs::create_dir(&staging).unwrap();
        fs::write(staging.join("new.txt"), "new").unwrap();

        replace_output(&dest, &staging).unwrap();

        assert!(!dest.join("old.txt").exists());
        assert_eq!(fs::read_to_string(dest.join("new.txt")).unwrap(), "new");
    }

    #[test]
    fn empty_dirs_are_not_files() {
        let staging = tempfile::tempdir().unwrap();
        fs::create_dir(staging.path().join("empty")).unwrap();
        assert!(!tree_has_files(staging.path()).unwrap());
    }

    #[test]
    fn refuses_project_root_and_ancestors() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir(&project).unwrap();
        let project = project.canonicalize().unwrap();

        let root_err = resolve_output_dir(&project, &project).unwrap_err();
        assert!(root_err
            .to_string()
            .contains("would replace the Dream project"));

        let ancestor_err = resolve_output_dir(&project, tmp.path()).unwrap_err();
        assert!(ancestor_err
            .to_string()
            .contains("would replace the Dream project"));

        let child = resolve_output_dir(&project, &project.join("gen")).unwrap();
        assert_eq!(child, project.join("gen"));
    }
}
