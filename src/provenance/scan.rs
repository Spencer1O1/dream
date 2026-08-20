use std::fs;
use std::path::{Component, Path};

use crate::error::DreamError;

pub fn has_user_files(dest: &Path) -> Result<bool, DreamError> {
    if !dest.exists() {
        return Ok(false);
    }
    walk_user_files(dest, dest)
}

fn walk_user_files(root: &Path, dir: &Path) -> Result<bool, DreamError> {
    if !dir.exists() {
        return Ok(false);
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let rel = output_rel(root, &path);
        if reserved(&rel) {
            continue;
        }
        if path.is_dir() {
            if walk_user_files(root, &path)? {
                return Ok(true);
            }
        } else {
            return Ok(true);
        }
    }
    Ok(false)
}

fn reserved(rel: &str) -> bool {
    rel == ".dream" || rel.starts_with(".dream/")
}

fn output_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn dream_dir_is_never_user_files() {
        let dest = tempfile::tempdir().unwrap();
        fs::create_dir_all(dest.path().join(".dream")).unwrap();
        fs::write(dest.path().join(".dream/provenance.json"), "{}").unwrap();
        assert!(!has_user_files(dest.path()).unwrap());
    }

    #[test]
    fn leftover_toolchain_output_is_occupied() {
        let dest = tempfile::tempdir().unwrap();
        fs::create_dir_all(dest.path().join("target/debug")).unwrap();
        fs::write(dest.path().join("target/debug/x"), "bin").unwrap();
        assert!(has_user_files(dest.path()).unwrap());
    }

    #[test]
    fn a_readme_is_a_user_file() {
        let dest = tempfile::tempdir().unwrap();
        fs::write(dest.path().join("README.md"), "hi").unwrap();
        assert!(has_user_files(dest.path()).unwrap());
    }
}
