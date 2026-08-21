use std::fs;
use std::path::{Path, PathBuf};

use crate::error::DreamError;

use super::manifest::{self, Manifest};
use super::paths::{rel_data, rel_path, resolve_data, resolve_inside};

#[derive(Debug, Clone)]
pub struct Unit {
    pub rel: String,
    pub source: String,
}

#[derive(Debug)]
pub struct Project {
    root: PathBuf,
    name: Option<String>,
    entry: Option<String>,
}

impl Project {
    pub fn from_path(path: &Path) -> Result<(Self, Unit), DreamError> {
        if path.is_dir() {
            Self::from_manifest(path)
        } else {
            Self::from_entry(path)
        }
    }

    pub fn from_entry(entry: &Path) -> Result<(Self, Unit), DreamError> {
        if entry.extension().and_then(|ext| ext.to_str()) != Some("foo") {
            return Err(DreamError::usage("expected a .foo file"));
        }
        if !entry.exists() {
            return Err(DreamError::usage(format!(
                "entry file `{}` does not exist",
                entry.display()
            )));
        }

        let entry_canon = entry.canonicalize()?;
        let root = entry_canon
            .parent()
            .ok_or_else(|| DreamError::usage("entry file has no project root"))?
            .to_path_buf();
        let rel = rel_path(&root, &entry_canon)?;
        let source = fs::read_to_string(&entry_canon)?;
        Ok((
            Self {
                root,
                name: None,
                entry: Some(rel.clone()),
            },
            Unit { rel, source },
        ))
    }

    pub fn from_manifest(dir: &Path) -> Result<(Self, Unit), DreamError> {
        if !dir.is_dir() {
            return Err(DreamError::usage("expected a directory"));
        }
        let root = dir.canonicalize()?;
        let Manifest { name, entry } = manifest::load(&root)?
            .ok_or_else(|| DreamError::usage("directory needs dream.toml with [project] entry"))?;
        let resolved = resolve_inside(&root, &entry)
            .map_err(|_| DreamError::usage("dream.toml entry escapes the project"))?;
        if !resolved.exists() {
            return Err(DreamError::usage(format!(
                "entry file `{entry}` does not exist"
            )));
        }
        let entry_canon = resolved.canonicalize()?;
        if !entry_canon.starts_with(&root) {
            return Err(DreamError::usage("dream.toml entry escapes the project"));
        }
        let rel = rel_path(&root, &entry_canon)?;
        let source = fs::read_to_string(&entry_canon)?;
        Ok((
            Self {
                root,
                name,
                entry: Some(rel.clone()),
            },
            Unit { rel, source },
        ))
    }

    pub fn from_root(dir: &Path) -> Result<Self, DreamError> {
        if !dir.is_dir() {
            return Err(DreamError::usage("expected a directory"));
        }
        let root = dir.canonicalize()?;
        let loaded = manifest::load(&root)?;
        Ok(Self {
            root,
            name: loaded.as_ref().and_then(|manifest| manifest.name.clone()),
            entry: loaded.map(|manifest| manifest.entry),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn entry(&self) -> Option<&str> {
        self.entry.as_deref()
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
            return Err(DreamError::runtime(format!(
                "requested source `{requested}` does not exist"
            )));
        }
        if resolved.extension().and_then(|ext| ext.to_str()) != Some("foo") {
            return Err(DreamError::runtime(format!(
                "requested source `{requested}` is not a .foo file"
            )));
        }
        let canon = resolved.canonicalize()?;
        if !canon.starts_with(&self.root) {
            return Err(DreamError::runtime("source request escapes project root"));
        }
        let rel = rel_path(&self.root, &canon)?;
        let source = fs::read_to_string(&canon)?;
        Ok(Unit { rel, source })
    }

    pub fn list_data_files(&self) -> Result<Vec<String>, DreamError> {
        let mut files = Vec::new();
        collect_data_files(&self.root, &self.root, &mut files)?;
        files.sort();
        Ok(files)
    }

    pub fn read_data_file(&self, requested: &str) -> Result<(String, String), DreamError> {
        let resolved = resolve_data(&self.root, requested)?;
        reject_source_or_dream(requested, false)?;
        if !resolved.exists() {
            return Err(DreamError::runtime(format!(
                "requested file `{requested}` does not exist"
            )));
        }
        if resolved.is_dir() {
            return Err(DreamError::runtime(format!(
                "requested file `{requested}` is a directory"
            )));
        }
        let canon = resolved.canonicalize()?;
        if !canon.starts_with(&self.root) {
            return Err(DreamError::runtime("file request escapes project root"));
        }
        let rel = rel_data(&self.root, &canon)?;
        let contents = fs::read_to_string(&canon).map_err(|err| {
            if err.kind() == std::io::ErrorKind::InvalidData {
                DreamError::runtime(format!("`{rel}` is not UTF-8"))
            } else {
                DreamError::runtime(err.to_string())
            }
        })?;
        Ok((rel, contents))
    }

    pub fn write_data_file(&self, requested: &str, contents: &str) -> Result<String, DreamError> {
        let resolved = resolve_data(&self.root, requested)?;
        reject_source_or_dream(requested, true)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&resolved, contents)?;
        rel_data(&self.root, &resolved)
    }
}

fn reject_source_or_dream(requested: &str, write: bool) -> Result<(), DreamError> {
    let path = Path::new(requested);
    if path.extension().and_then(|ext| ext.to_str()) == Some("foo") {
        return Err(DreamError::runtime(if write {
            "cannot write a `.foo` file"
        } else {
            "use read_source_file for a `.foo` file"
        }));
    }
    let first = path
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str());
    if first == Some(".dream") {
        return Err(DreamError::runtime("`.dream` is not a data file"));
    }
    Ok(())
}

fn collect_foo_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), DreamError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
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

fn collect_data_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), DreamError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            if name == ".dream" {
                continue;
            }
            collect_data_files(root, &path, out)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("foo") {
            continue;
        }
        let canon = path.canonicalize()?;
        out.push(rel_data(root, &canon)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_foo(dir: &Path, rel: &str, contents: &str) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn lists_project_foo_files() {
        let tmp = tempfile::tempdir().unwrap();
        write_foo(tmp.path(), "main.foo", "entry");
        write_foo(tmp.path(), "users/active.foo", "active");
        write_foo(tmp.path(), "readme.md", "no");
        write_foo(tmp.path(), ".hidden/secret.foo", "dot");
        write_foo(tmp.path(), "target/gen.foo", "gen");
        let (project, unit) = Project::from_entry(&tmp.path().join("main.foo")).unwrap();
        assert_eq!(unit.rel, "main.foo");
        assert_eq!(
            project.list_source_files().unwrap(),
            vec![
                ".hidden/secret.foo".to_string(),
                "main.foo".to_string(),
                "target/gen.foo".to_string(),
                "users/active.foo".to_string()
            ]
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

    #[test]
    fn directory_uses_dream_toml_entry() {
        let tmp = tempfile::tempdir().unwrap();
        write_foo(tmp.path(), "src/app.foo", "from toml");
        write_foo(tmp.path(), "other.foo", "sibling");
        fs::write(
            tmp.path().join("dream.toml"),
            "[project]\nname = \"demo\"\nentry = \"src/app.foo\"\n",
        )
        .unwrap();
        let (project, unit) = Project::from_path(tmp.path()).unwrap();
        assert_eq!(unit.rel, "src/app.foo");
        assert_eq!(unit.source, "from toml");
        assert_eq!(project.name(), Some("demo"));
        assert_eq!(
            project.list_source_files().unwrap(),
            vec!["other.foo".to_string(), "src/app.foo".to_string()]
        );
    }

    #[test]
    fn directory_without_toml_is_usage() {
        let tmp = tempfile::tempdir().unwrap();
        write_foo(tmp.path(), "main.foo", "entry");
        let err = Project::from_path(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("dream.toml"));
    }

    #[test]
    fn lists_and_writes_data_files() {
        let tmp = tempfile::tempdir().unwrap();
        write_foo(tmp.path(), "main.foo", "entry");
        fs::write(tmp.path().join("users.json"), "[1]").unwrap();
        fs::create_dir_all(tmp.path().join(".dream")).unwrap();
        fs::write(tmp.path().join(".dream/provenance.json"), "{}").unwrap();
        let (project, _) = Project::from_entry(&tmp.path().join("main.foo")).unwrap();
        assert_eq!(project.list_data_files().unwrap(), vec!["users.json"]);
        let (rel, contents) = project.read_data_file("users.json").unwrap();
        assert_eq!((rel.as_str(), contents.as_str()), ("users.json", "[1]"));
        let written = project.write_data_file("out/note.txt", "hi").unwrap();
        assert_eq!(written, "out/note.txt");
        assert_eq!(
            fs::read_to_string(tmp.path().join("out/note.txt")).unwrap(),
            "hi"
        );
        let foo = project.read_data_file("main.foo").unwrap_err();
        assert!(foo.to_string().contains("read_source_file"));
        let dream = project.write_data_file(".dream/x", "no").unwrap_err();
        assert!(dream.to_string().contains(".dream"));
    }
}
