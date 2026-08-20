use std::collections::HashSet;
use std::fs;
use std::path::Path;

use toml_edit::{value, Array, InlineTable, Item, Table, Value};

use crate::error::DreamError;
use crate::provenance::Dependency;

pub fn create_if_missing(dest: &Path, package: &str) -> Result<(), DreamError> {
    let path = dest.join("Cargo.toml");
    if path.exists() {
        return Ok(());
    }
    let mut doc = toml_edit::DocumentMut::new();
    doc["package"]["name"] = value(package);
    doc["package"]["version"] = value("0.1.0");
    doc["package"]["edition"] = value("2021");
    doc["dependencies"] = Item::Table(Table::new());
    fs::write(path, doc.to_string())?;
    Ok(())
}

pub fn apply(
    dest: &Path,
    wanted: &[Dependency],
    installed: &mut Vec<String>,
) -> Result<(), DreamError> {
    let path = dest.join("Cargo.toml");
    let mut doc = fs::read_to_string(&path)?
        .parse::<toml_edit::DocumentMut>()
        .map_err(|err| DreamError::composer(format!("invalid Cargo.toml: {err}")))?;
    if doc.get("dependencies").is_none() {
        doc["dependencies"] = Item::Table(Table::new());
    }
    let deps = doc["dependencies"]
        .as_table_mut()
        .ok_or_else(|| DreamError::composer("Cargo.toml [dependencies] must be a table"))?;

    let existing: HashSet<String> = deps.iter().map(|(name, _)| name.to_string()).collect();
    let wanted_names: HashSet<&str> = wanted.iter().map(|dep| dep.name.as_str()).collect();
    let mut installed_set: HashSet<String> = installed.iter().cloned().collect();

    for dep in wanted {
        if existing.contains(&dep.name) {
            if installed_set.contains(&dep.name) {
                deps[&dep.name] = dep_item(dep);
            }
            continue;
        }
        deps[&dep.name] = dep_item(dep);
        installed_set.insert(dep.name.clone());
    }

    for name in installed.iter() {
        if !wanted_names.contains(name.as_str()) {
            deps.remove(name);
            installed_set.remove(name);
        }
    }

    let mut next: Vec<String> = installed_set.into_iter().collect();
    next.sort();
    *installed = next;
    fs::write(path, doc.to_string())?;
    Ok(())
}

fn dep_item(dep: &Dependency) -> Item {
    let version = dep.version.as_deref().unwrap_or("*");
    if dep.features.is_empty() {
        return value(version);
    }
    Item::Value(Value::InlineTable(feature_table(version, &dep.features)))
}

fn feature_table(version: &str, features: &[String]) -> InlineTable {
    let mut table = InlineTable::new();
    table.insert("version", version.into());
    table.insert("features", Value::Array(feature_array(features)));
    table
}

fn feature_array(features: &[String]) -> Array {
    let mut array = Array::new();
    for feature in features {
        array.push(feature.as_str());
    }
    array
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn dep(name: &str, features: &[&str]) -> Dependency {
        Dependency {
            name: name.into(),
            version: None,
            features: features
                .iter()
                .map(|feature| (*feature).to_string())
                .collect(),
        }
    }

    #[test]
    fn create_does_not_rename() {
        let dest = tempfile::tempdir().unwrap();
        create_if_missing(dest.path(), "from-entry").unwrap();
        fs::write(
            dest.path().join("Cargo.toml"),
            "[package]\nname = \"renamed\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        create_if_missing(dest.path(), "from-entry").unwrap();
        let text = fs::read_to_string(dest.path().join("Cargo.toml")).unwrap();
        assert!(text.contains("name = \"renamed\""));
        assert!(!text.contains("from-entry"));
    }

    #[test]
    fn adds_retracts_dream_deps_and_keeps_user_deps() {
        let dest = tempfile::tempdir().unwrap();
        create_if_missing(dest.path(), "demo").unwrap();
        let mut toml = fs::read_to_string(dest.path().join("Cargo.toml")).unwrap();
        toml.push_str("clap = \"4\"\n");
        fs::write(dest.path().join("Cargo.toml"), toml).unwrap();

        let mut installed = Vec::new();
        apply(dest.path(), &[dep("serde", &["derive"])], &mut installed).unwrap();
        assert_eq!(installed, vec!["serde"]);
        let text = fs::read_to_string(dest.path().join("Cargo.toml")).unwrap();
        assert!(text.contains("serde"));
        assert!(text.contains("derive"));
        assert!(text.contains("clap"));

        apply(dest.path(), &[], &mut installed).unwrap();
        assert!(installed.is_empty());
        let text = fs::read_to_string(dest.path().join("Cargo.toml")).unwrap();
        assert!(!text.contains("serde"));
        assert!(text.contains("clap"));
    }

    #[test]
    fn writes_optional_version() {
        let dest = tempfile::tempdir().unwrap();
        create_if_missing(dest.path(), "demo").unwrap();
        let mut installed = Vec::new();
        apply(
            dest.path(),
            &[Dependency {
                name: "serde".into(),
                version: Some("1.0".into()),
                features: vec!["derive".into()],
            }],
            &mut installed,
        )
        .unwrap();
        let text = fs::read_to_string(dest.path().join("Cargo.toml")).unwrap();
        assert!(text.contains("1.0"));
        assert!(text.contains("derive"));
    }
}
