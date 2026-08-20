use std::path::{Component, Path, PathBuf};

use crate::error::DreamError;

pub fn resolve_inside(root: &Path, requested: &str) -> Result<PathBuf, DreamError> {
    if requested.trim().is_empty() {
        return Err(DreamError::runtime("source request is empty"));
    }
    let requested_path = Path::new(requested);
    let joined = if requested_path.is_absolute() {
        requested_path.to_path_buf()
    } else {
        root.join(requested_path)
    };
    let normalized = normalize_lexically(&joined);
    if !normalized.starts_with(root) {
        return Err(DreamError::runtime("source request escapes project root"));
    }
    Ok(normalized)
}

pub fn rel_path(root: &Path, path: &Path) -> Result<String, DreamError> {
    let rel = path
        .strip_prefix(root)
        .map_err(|_| DreamError::runtime("source request escapes project root"))?;
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
