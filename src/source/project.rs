use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::DreamError;

#[derive(Debug, Clone)]
pub struct Unit {
    pub rel: String,
    pub source: String,
}

#[derive(Debug)]
pub struct Project {
    root: PathBuf,
}

impl Project {
    pub fn from_entry(entry: &Path) -> Result<(Self, Unit), DreamError> {
        if entry.extension().and_then(|ext| ext.to_str()) != Some("foo") {
            return Err(DreamError::new("expected a .foo file"));
        }
        if !entry.exists() {
            return Err(DreamError::new(format!(
                "entry file `{}` does not exist",
                entry.display()
            )));
        }

        let entry_canon = entry.canonicalize()?;
        let root = entry_canon
            .parent()
            .ok_or_else(|| DreamError::new("entry file has no project root"))?
            .to_path_buf();
        let rel = rel_path(&root, &entry_canon)?;
        let source = fs::read_to_string(&entry_canon)?;
        Ok((Self { root }, Unit { rel, source }))
    }

    pub fn list_source_files(&self) -> Result<Vec<String>, DreamError> {
        let mut files = Vec::new();
        collect_foo_files(&self.root, &self.root, &mut files)?;
        files.sort();
        Ok(files)
    }

    pub fn read_source_file(&self, requested: &str) -> Result<Unit, DreamError> {
        let resolved = resolve_inside(&self.root, requested)?;
        if !resolved.exists() {
            return Err(DreamError::new(format!(
                "requested source `{requested}` does not exist"
            )));
        }
        if resolved.extension().and_then(|ext| ext.to_str()) != Some("foo") {
            return Err(DreamError::new(format!(
                "requested source `{requested}` is not a .foo file"
            )));
        }
        let canon = resolved.canonicalize()?;
        if !canon.starts_with(&self.root) {
            return Err(DreamError::new("source request escapes project root"));
        }
        let rel = rel_path(&self.root, &canon)?;
        let source = fs::read_to_string(&canon)?;
        Ok(Unit { rel, source })
    }
}

fn collect_foo_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), DreamError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_foo_files(root, &path, out)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("foo") {
            let canon = path.canonicalize()?;
            out.push(rel_path(root, &canon)?);
        }
    }
    Ok(())
}

fn resolve_inside(root: &Path, requested: &str) -> Result<PathBuf, DreamError> {
    if requested.trim().is_empty() {
        return Err(DreamError::new("source request is empty"));
    }
    let requested_path = Path::new(requested);
    let joined = if requested_path.is_absolute() {
        requested_path.to_path_buf()
    } else {
        root.join(requested_path)
    };
    let normalized = normalize_lexically(&joined);
    if !normalized.starts_with(root) {
        return Err(DreamError::new("source request escapes project root"));
    }
    Ok(normalized)
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

fn rel_path(root: &Path, path: &Path) -> Result<String, DreamError> {
    let rel = path
        .strip_prefix(root)
        .map_err(|_| DreamError::new("source request escapes project root"))?;
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn write_foo(dir: &Path, rel: &str, contents: &str) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn lists_project_foo_files() {
        let tmp = tempfile::tempdir().unwrap();
        write_foo(tmp.path(), "main.foo", "entry");
        write_foo(tmp.path(), "users/active.foo", "active");
        write_foo(tmp.path(), "readme.md", "no");
        let (project, unit) = Project::from_entry(&tmp.path().join("main.foo")).unwrap();
        assert_eq!(unit.rel, "main.foo");
        assert_eq!(
            project.list_source_files().unwrap(),
            vec!["main.foo".to_string(), "users/active.foo".to_string()]
        );
    }

    #[test]
    fn reads_nested_unit() {
        let tmp = tempfile::tempdir().unwrap();
        write_foo(tmp.path(), "main.foo", "entry");
        write_foo(tmp.path(), "users/active.foo", "active users");
        let (project, _) = Project::from_entry(&tmp.path().join("main.foo")).unwrap();
        let unit = project.read_source_file("users/active.foo").unwrap();
        assert_eq!(unit.rel, "users/active.foo");
        assert_eq!(unit.source, "active users");
    }

    #[test]
    fn rejects_missing_and_escape() {
        let tmp = tempfile::tempdir().unwrap();
        write_foo(tmp.path(), "main.foo", "entry");
        let (project, _) = Project::from_entry(&tmp.path().join("main.foo")).unwrap();
        let missing = project.read_source_file("nope.foo").unwrap_err();
        assert!(missing.to_string().contains("does not exist"));
        let escape = project.read_source_file("../secret.foo").unwrap_err();
        assert!(escape.to_string().contains("escapes project root"));
    }
}
