use std::path::{Component, Path, PathBuf};

use crate::error::DreamError;

pub fn resolve_inside(root: &Path, requested: &str) -> Result<PathBuf, DreamError> {
    resolve_under(
        root,
        requested,
        "source request is empty",
        "source request escapes project root",
    )
}

pub fn resolve_output(root: &Path, requested: &str) -> Result<PathBuf, DreamError> {
    let dest = resolve_under(
        root,
        requested,
        "output write is empty",
        "output write escapes -o",
    )?;
    if dest == root {
        return Err(DreamError::runtime("output write escapes -o"));
    }
    Ok(dest)
}

fn resolve_under(
    root: &Path,
    requested: &str,
    empty: &str,
    escape: &str,
) -> Result<PathBuf, DreamError> {
    if requested.trim().is_empty() {
        return Err(DreamError::runtime(empty));
    }
    let requested_path = Path::new(requested);
    let joined = if requested_path.is_absolute() {
        requested_path.to_path_buf()
    } else {
        root.join(requested_path)
    };
    let normalized = normalize_lexically(&joined);
    if !normalized.starts_with(root) {
        return Err(DreamError::runtime(escape));
    }
    Ok(normalized)
}

pub fn rel_path(root: &Path, path: &Path) -> Result<String, DreamError> {
    rel_under(root, path, "source request escapes project root")
}

pub fn rel_output(root: &Path, path: &Path) -> Result<String, DreamError> {
    rel_under(root, path, "output write escapes -o")
}

fn rel_under(root: &Path, path: &Path, escape: &str) -> Result<String, DreamError> {
    let rel = path
        .strip_prefix(root)
        .map_err(|_| DreamError::runtime(escape))?;
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_stays_under_root() {
        let root = Path::new("/tmp/stage");
        let dest = resolve_output(root, "src/main.rs").unwrap();
        assert_eq!(dest, PathBuf::from("/tmp/stage/src/main.rs"));
    }

    #[test]
    fn output_rejects_escape_and_root() {
        let root = Path::new("/tmp/stage");
        let escape = resolve_output(root, "../secret").unwrap_err();
        assert!(escape.to_string().contains("output write escapes -o"));
        let abs = resolve_output(root, "/etc/passwd").unwrap_err();
        assert!(abs.to_string().contains("output write escapes -o"));
        let root_write = resolve_output(root, ".").unwrap_err();
        assert!(root_write.to_string().contains("output write escapes -o"));
        let empty = resolve_output(root, "  ").unwrap_err();
        assert!(empty.to_string().contains("output write is empty"));
    }

    #[test]
    fn source_still_rejects_escape() {
        let root = Path::new("/tmp/proj");
        let err = resolve_inside(root, "../secret.foo").unwrap_err();
        assert!(err
            .to_string()
            .contains("source request escapes project root"));
    }
}
