use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::DreamError;

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
}
