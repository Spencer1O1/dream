use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::DreamError;

use crate::output;

pub const STORE_REL: &str = ".dream/provenance.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Store {
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_root: Option<String>,
    #[serde(default)]
    pub units: BTreeMap<String, UnitState>,
    #[serde(default)]
    pub project: Vec<String>,
    #[serde(default)]
    pub installed: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitState {
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owner {
    Unit(String),
    Project,
    Unmanaged,
}

impl Store {
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            source_root: None,
            units: BTreeMap::new(),
            project: Vec::new(),
            installed: Vec::new(),
        }
    }

    pub fn set_source_root(&mut self, root: &Path) -> Result<(), DreamError> {
        let root = root.canonicalize()?;
        self.source_root = Some(root.to_string_lossy().into_owned());
        Ok(())
    }

    pub fn path(dest: &Path) -> std::path::PathBuf {
        dest.join(STORE_REL)
    }

    pub fn load(dest: &Path) -> Result<Option<Self>, DreamError> {
        let path = Self::path(dest);
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path)?;
        let store: Self = serde_json::from_str(&text)
            .map_err(|err| DreamError::runtime(format!("invalid provenance store: {err}")))?;
        Ok(Some(store))
    }

    pub fn save(&self, dest: &Path) -> Result<(), DreamError> {
        let path = Self::path(dest);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn owner(&self, rel: &str) -> Owner {
        if reserved(rel) || self.project.iter().any(|path| path == rel) {
            return Owner::Project;
        }
        for (unit, state) in &self.units {
            if state.artifacts.iter().any(|artifact| artifact == rel) {
                return Owner::Unit(unit.clone());
            }
        }
        Owner::Unmanaged
    }

    pub fn set_artifacts(&mut self, unit: &str, artifacts: HashSet<String>) {
        let dependencies = self
            .units
            .get(unit)
            .map(|state| state.dependencies.clone())
            .unwrap_or_default();
        write_unit(self, unit, sorted_paths(artifacts), dependencies);
    }

    pub fn set_dependencies(&mut self, unit: &str, dependencies: Vec<Dependency>) {
        let artifacts = self
            .units
            .get(unit)
            .map(|state| state.artifacts.clone())
            .unwrap_or_default();
        write_unit(self, unit, artifacts, dependencies);
    }

    pub fn take_from_units(&mut self, rel: &str) {
        for state in self.units.values_mut() {
            state.artifacts.retain(|artifact| artifact != rel);
        }
        self.units.retain(|_, state| {
            !state.artifacts.is_empty() || !state.dependencies.is_empty() || state.locked
        });
    }

    pub fn mark_project(&mut self, rel: &str) {
        self.take_from_units(rel);
        if !self.project.iter().any(|path| path == rel) {
            self.project.push(rel.to_string());
            self.project.sort();
        }
    }

    pub fn union_dependencies(&self) -> Result<Vec<Dependency>, DreamError> {
        struct Acc {
            version: Option<String>,
            features: BTreeSet<String>,
        }
        let mut acc: BTreeMap<String, Acc> = BTreeMap::new();
        for state in self.units.values() {
            for dep in &state.dependencies {
                let entry = acc.entry(dep.name.clone()).or_insert(Acc {
                    version: None,
                    features: BTreeSet::new(),
                });
                if let Some(version) = &dep.version {
                    match &entry.version {
                        None => entry.version = Some(version.clone()),
                        Some(prev) if prev != version => {
                            return Err(DreamError::composer(format!(
                                "dependency `{}` has conflicting versions `{prev}` and `{version}`",
                                dep.name
                            )));
                        }
                        Some(_) => {}
                    }
                }
                entry.features.extend(dep.features.iter().cloned());
            }
        }
        Ok(acc
            .into_iter()
            .map(|(name, acc)| Dependency {
                name,
                version: acc.version,
                features: acc.features.into_iter().collect(),
            })
            .collect())
    }

    pub fn is_locked(&self, unit: &str) -> bool {
        self.units.get(unit).is_some_and(|state| state.locked)
    }

    pub fn set_lock(&mut self, unit: &str, source_hash: String) {
        if let Some(state) = self.units.get_mut(unit) {
            state.locked = true;
            state.source_hash = Some(source_hash);
        }
    }

    pub fn clear_lock(&mut self, unit: &str) {
        if let Some(state) = self.units.get_mut(unit) {
            state.locked = false;
            state.source_hash = None;
        }
    }

    pub fn drop_owned(&self, dest: &Path) -> Result<(), DreamError> {
        for path in &self.project {
            let _ = output::remove_file(dest, path);
        }
        for state in self.units.values() {
            for artifact in &state.artifacts {
                let _ = output::remove_file(dest, artifact);
            }
        }
        let dream_dir = dest.join(".dream");
        if dream_dir.exists() {
            fs::remove_dir_all(dream_dir)?;
        }
        Ok(())
    }
}

fn write_unit(
    store: &mut Store,
    unit: &str,
    artifacts: Vec<String>,
    dependencies: Vec<Dependency>,
) {
    let existing = store.units.get(unit);
    let locked = existing.is_some_and(|state| state.locked);
    let source_hash = existing.and_then(|state| state.source_hash.clone());
    if artifacts.is_empty() && dependencies.is_empty() && !locked {
        store.units.remove(unit);
    } else {
        store.units.insert(
            unit.to_string(),
            UnitState {
                artifacts,
                dependencies,
                locked,
                source_hash,
            },
        );
    }
}

fn sorted_paths(artifacts: HashSet<String>) -> Vec<String> {
    let mut artifacts: Vec<String> = artifacts.into_iter().collect();
    artifacts.sort();
    artifacts
}

pub fn reserved(rel: &str) -> bool {
    rel == ".dream" || rel.starts_with(".dream/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_is_the_dream_dir() {
        assert!(reserved(".dream/provenance.json"));
        assert!(reserved(".dream"));
        assert!(!reserved("src/main.rs"));
        assert!(!reserved("dream.lock"));
    }

    #[test]
    fn owner_prefers_project_then_unit_then_unmanaged() {
        let mut store = Store::new("rust");
        store.set_artifacts(
            "server.foo",
            HashSet::from(["src/server.rs".into(), "src/routes.rs".into()]),
        );
        store.mark_project("Cargo.toml");
        assert_eq!(
            store.owner("src/server.rs"),
            Owner::Unit("server.foo".into())
        );
        assert_eq!(store.owner("Cargo.toml"), Owner::Project);
        assert_eq!(store.owner("README.md"), Owner::Unmanaged);
        assert_eq!(store.owner(".dream/provenance.json"), Owner::Project);
    }

    #[test]
    fn mark_project_steals_from_units() {
        let mut store = Store::new("rust");
        store.set_artifacts(
            "main.foo",
            HashSet::from(["Cargo.toml".into(), "src/main.rs".into()]),
        );
        store.mark_project("Cargo.toml");
        assert_eq!(store.units["main.foo"].artifacts, vec!["src/main.rs"]);
        assert_eq!(store.project, vec!["Cargo.toml"]);
        assert_eq!(store.owner("Cargo.toml"), Owner::Project);
    }

    #[test]
    fn set_artifacts_keeps_dependencies() {
        let mut store = Store::new("rust");
        store.set_dependencies(
            "main.foo",
            vec![Dependency {
                name: "serde".into(),
                version: None,
                features: vec!["derive".into()],
            }],
        );
        store.set_artifacts("main.foo", HashSet::from(["src/main.rs".into()]));
        assert_eq!(store.units["main.foo"].dependencies[0].name, "serde");
    }

    #[test]
    fn set_artifacts_keeps_a_lock() {
        let mut store = Store::new("rust");
        store.set_artifacts("main.foo", HashSet::from(["src/main.rs".into()]));
        store.set_lock("main.foo", "abc".into());
        store.set_artifacts("main.foo", HashSet::from(["src/lib.rs".into()]));
        assert!(store.is_locked("main.foo"));
        assert_eq!(store.units["main.foo"].source_hash.as_deref(), Some("abc"));
        store.set_artifacts("main.foo", HashSet::new());
        assert!(store.is_locked("main.foo"));
        store.take_from_units("src/lib.rs");
        assert!(store.is_locked("main.foo"));
    }

    #[test]
    fn drop_owned_deletes_project_and_unit_files() {
        let dest = tempfile::tempdir().unwrap();
        std::fs::write(dest.path().join("Cargo.toml"), "[package]\n").unwrap();
        std::fs::create_dir_all(dest.path().join("src")).unwrap();
        std::fs::write(dest.path().join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(dest.path().join("README.md"), "keep").unwrap();
        let mut store = Store::new("rust");
        store.mark_project("Cargo.toml");
        store.set_artifacts("main.foo", HashSet::from(["src/main.rs".into()]));
        store.save(dest.path()).unwrap();
        store.drop_owned(dest.path()).unwrap();
        assert!(!dest.path().join("Cargo.toml").exists());
        assert!(!dest.path().join("src/main.rs").exists());
        assert!(dest.path().join("README.md").exists());
        assert!(!Store::path(dest.path()).exists());
    }

    #[test]
    fn union_dependencies_merges_features() {
        let mut store = Store::new("rust");
        store.set_dependencies(
            "a.foo",
            vec![Dependency {
                name: "serde".into(),
                version: None,
                features: vec!["derive".into()],
            }],
        );
        store.set_dependencies(
            "b.foo",
            vec![
                Dependency {
                    name: "serde".into(),
                    version: None,
                    features: vec!["rc".into()],
                },
                Dependency {
                    name: "tokio".into(),
                    version: None,
                    features: vec![],
                },
            ],
        );
        let union = store.union_dependencies().unwrap();
        assert_eq!(union[0].name, "serde");
        assert_eq!(union[0].features, vec!["derive", "rc"]);
        assert_eq!(union[1].name, "tokio");
    }

    #[test]
    fn union_uses_the_specified_version() {
        let mut store = Store::new("rust");
        store.set_dependencies(
            "a.foo",
            vec![Dependency {
                name: "serde".into(),
                version: Some("1.0".into()),
                features: vec![],
            }],
        );
        store.set_dependencies(
            "b.foo",
            vec![Dependency {
                name: "serde".into(),
                version: None,
                features: vec!["derive".into()],
            }],
        );
        let union = store.union_dependencies().unwrap();
        assert_eq!(union[0].version.as_deref(), Some("1.0"));
        assert_eq!(union[0].features, vec!["derive"]);
    }

    #[test]
    fn union_rejects_conflicting_versions() {
        let mut store = Store::new("rust");
        store.set_dependencies(
            "a.foo",
            vec![Dependency {
                name: "serde".into(),
                version: Some("1.0".into()),
                features: vec![],
            }],
        );
        store.set_dependencies(
            "b.foo",
            vec![Dependency {
                name: "serde".into(),
                version: Some("1.0.200".into()),
                features: vec![],
            }],
        );
        let err = store.union_dependencies().unwrap_err();
        assert!(err.to_string().contains("conflicting versions"));
    }
}
