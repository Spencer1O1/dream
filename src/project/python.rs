use std::collections::HashSet;
use std::fs;
use std::path::Path;

use toml_edit::{value, Array, Item, Value};

use crate::composer::provenance::Dependency;
use crate::error::DreamError;

pub fn create_if_missing(dest: &Path, package: &str) -> Result<(), DreamError> {
    let path = dest.join("pyproject.toml");
    if path.exists() {
        return Ok(());
    }
    let mut doc = toml_edit::DocumentMut::new();
    doc["project"]["name"] = value(package);
    doc["project"]["version"] = value("0.1.0");
    doc["project"]["dependencies"] = Item::Value(Value::Array(Array::new()));
    fs::write(path, doc.to_string())?;
    Ok(())
}

pub fn apply(
    dest: &Path,
    wanted: &[Dependency],
    installed: &mut Vec<String>,
) -> Result<(), DreamError> {
    let path = dest.join("pyproject.toml");
    let mut doc = fs::read_to_string(&path)?
        .parse::<toml_edit::DocumentMut>()
        .map_err(|err| DreamError::runtime(format!("invalid pyproject.toml: {err}")))?;
    if doc
        .get("project")
        .and_then(|project| project.get("dependencies"))
        .is_none()
    {
        doc["project"]["dependencies"] = Item::Value(Value::Array(Array::new()));
    }
    let deps = doc["project"]["dependencies"]
        .as_array_mut()
        .ok_or_else(|| {
            DreamError::runtime("pyproject.toml project.dependencies must be an array")
        })?;

    let existing: HashSet<String> = deps
        .iter()
        .filter_map(Value::as_str)
        .map(requirement_name)
        .collect();
    let wanted_names: HashSet<&str> = wanted.iter().map(|dep| dep.name.as_str()).collect();
    let mut installed_set: HashSet<String> = installed.iter().cloned().collect();

    for dep in wanted {
        if existing.contains(&dep.name) {
            if installed_set.contains(&dep.name) {
                replace_requirement(deps, dep);
            }
            continue;
        }
        deps.push(requirement(dep));
        installed_set.insert(dep.name.clone());
    }

    for name in installed.iter() {
        if !wanted_names.contains(name.as_str()) {
            remove_requirement(deps, name);
            installed_set.remove(name);
        }
    }

    let mut next: Vec<String> = installed_set.into_iter().collect();
    next.sort();
    *installed = next;
    fs::write(path, doc.to_string())?;
    Ok(())
}

fn requirement(dep: &Dependency) -> String {
    if dep.features.is_empty() {
        dep.name.clone()
    } else {
        format!("{}[{}]", dep.name, dep.features.join(","))
    }
}

fn requirement_name(entry: &str) -> String {
    let end = entry
        .find(['[', ' ', '>', '<', '=', '~', '!'])
        .unwrap_or(entry.len());
    entry[..end].to_string()
}

fn replace_requirement(deps: &mut Array, dep: &Dependency) {
    for item in deps.iter_mut() {
        if item
            .as_str()
            .is_some_and(|entry| requirement_name(entry) == dep.name)
        {
            *item = requirement(dep).into();
            return;
        }
    }
}

fn remove_requirement(deps: &mut Array, name: &str) {
    let mut keep = Array::new();
    for item in deps.iter() {
        let Some(entry) = item.as_str() else {
            keep.push(item.clone());
            continue;
        };
        if requirement_name(entry) != name {
            keep.push(item.clone());
        }
    }
    *deps = keep;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn dep(name: &str, features: &[&str]) -> Dependency {
        Dependency {
            name: name.into(),
            features: features
                .iter()
                .map(|feature| (*feature).to_string())
                .collect(),
        }
    }

    #[test]
    fn adds_extras_and_keeps_user_requirements() {
        let dest = tempfile::tempdir().unwrap();
        create_if_missing(dest.path(), "demo").unwrap();
        let mut installed = Vec::new();
        apply(dest.path(), &[dep("httpx", &["http2"])], &mut installed).unwrap();
        let text = fs::read_to_string(dest.path().join("pyproject.toml")).unwrap();
        assert!(text.contains("httpx[http2]"));
        assert_eq!(installed, vec!["httpx"]);
    }
}
