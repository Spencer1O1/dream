use std::fs;
use std::path::Path;

use toml_edit::DocumentMut;

use crate::error::DreamError;

const FILE: &str = "dream.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub name: Option<String>,
    pub entry: String,
}

pub fn load(dir: &Path) -> Result<Option<Manifest>, DreamError> {
    let path = dir.join(FILE);
    if !path.exists() {
        return Ok(None);
    }
    let doc = fs::read_to_string(&path)?
        .parse::<DocumentMut>()
        .map_err(|err| DreamError::usage(format!("invalid dream.toml: {err}")))?;
    let project = doc
        .get("project")
        .ok_or_else(|| DreamError::usage("dream.toml needs [project]"))?;
    let entry = project
        .get("entry")
        .and_then(|value| value.as_str())
        .ok_or_else(|| DreamError::usage("dream.toml [project] needs entry"))?;
    if Path::new(entry).extension().and_then(|ext| ext.to_str()) != Some("foo") {
        return Err(DreamError::usage("dream.toml entry must be a .foo file"));
    }
    if entry.trim().is_empty() {
        return Err(DreamError::usage("dream.toml [project] needs entry"));
    }
    let name = project
        .get("name")
        .and_then(|value| value.as_str())
        .map(|name| name.to_string())
        .filter(|name| !name.is_empty());
    Ok(Some(Manifest {
        name,
        entry: entry.to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn missing_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).unwrap(), None);
    }

    #[test]
    fn reads_name_and_entry() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(FILE),
            "[project]\nname = \"demo\"\nentry = \"src/app.foo\"\n",
        )
        .unwrap();
        assert_eq!(
            load(dir.path()).unwrap(),
            Some(Manifest {
                name: Some("demo".into()),
                entry: "src/app.foo".into(),
            })
        );
    }

    #[test]
    fn rejects_a_non_foo_entry() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(FILE), "[project]\nentry = \"main.rs\"\n").unwrap();
        let err = load(dir.path()).unwrap_err();
        assert!(err.to_string().contains("must be a .foo file"));
    }

    #[test]
    fn rejects_missing_entry() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(FILE), "[project]\nname = \"demo\"\n").unwrap();
        let err = load(dir.path()).unwrap_err();
        assert!(err.to_string().contains("needs entry"));
    }
}
