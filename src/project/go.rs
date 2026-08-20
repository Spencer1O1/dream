use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::error::DreamError;
use crate::provenance::Dependency;

pub fn create_if_missing(dest: &Path, package: &str) -> Result<(), DreamError> {
    let path = dest.join("go.mod");
    if path.exists() {
        return Ok(());
    }
    fs::write(path, format!("module {package}\n"))?;
    Ok(())
}

pub fn apply(
    dest: &Path,
    wanted: &[Dependency],
    installed: &mut Vec<String>,
) -> Result<(), DreamError> {
    if wanted.iter().any(|dep| !dep.features.is_empty()) {
        return Err(DreamError::composer("go dependencies do not take features"));
    }
    let path = dest.join("go.mod");
    let mut text = fs::read_to_string(&path)?;
    let existing = require_names(&text);
    let wanted_names: HashSet<&str> = wanted.iter().map(|dep| dep.name.as_str()).collect();
    let mut installed_set: HashSet<String> = installed.iter().cloned().collect();

    for name in installed.iter() {
        if !wanted_names.contains(name.as_str()) {
            text = remove_require(&text, name);
            installed_set.remove(name);
        }
    }

    for dep in wanted {
        if existing.contains(&dep.name) {
            continue;
        }
        if !text.ends_with('\n') && !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&format!("require {} v0.0.0\n", dep.name));
        installed_set.insert(dep.name.clone());
    }

    let mut next: Vec<String> = installed_set.into_iter().collect();
    next.sort();
    *installed = next;
    fs::write(path, text)?;
    Ok(())
}

fn require_names(text: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "require (" {
            in_block = true;
            continue;
        }
        if in_block {
            if trimmed == ")" {
                in_block = false;
                continue;
            }
            if let Some(name) = require_name(trimmed) {
                names.insert(name);
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("require ") {
            if rest != "(" {
                if let Some(name) = require_name(rest) {
                    names.insert(name);
                }
            }
        }
    }
    names
}

fn require_name(spec: &str) -> Option<String> {
    spec.split_whitespace()
        .next()
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
}

fn remove_require(text: &str, name: &str) -> String {
    let mut out = String::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "require (" {
            in_block = true;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_block {
            if trimmed == ")" {
                in_block = false;
                out.push_str(line);
                out.push('\n');
                continue;
            }
            if require_name(trimmed).as_deref() == Some(name) {
                continue;
            }
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("require ") {
            if rest != "(" && require_name(rest).as_deref() == Some(name) {
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn dep(name: &str) -> Dependency {
        Dependency {
            name: name.into(),
            features: Vec::new(),
        }
    }

    #[test]
    fn adds_and_retracts_require_lines() {
        let dest = tempfile::tempdir().unwrap();
        create_if_missing(dest.path(), "demo").unwrap();
        let mut installed = Vec::new();
        apply(dest.path(), &[dep("example.com/foo")], &mut installed).unwrap();
        let text = fs::read_to_string(dest.path().join("go.mod")).unwrap();
        assert!(text.contains("module demo"));
        assert!(text.contains("require example.com/foo v0.0.0"));
        apply(dest.path(), &[], &mut installed).unwrap();
        let text = fs::read_to_string(dest.path().join("go.mod")).unwrap();
        assert!(!text.contains("example.com/foo"));
        assert!(text.contains("module demo"));
    }

    #[test]
    fn rejects_features() {
        let dest = tempfile::tempdir().unwrap();
        create_if_missing(dest.path(), "demo").unwrap();
        let mut installed = Vec::new();
        let err = apply(
            dest.path(),
            &[Dependency {
                name: "example.com/foo".into(),
                features: vec!["x".into()],
            }],
            &mut installed,
        )
        .unwrap_err();
        assert!(err.to_string().contains("do not take features"));
    }
}
