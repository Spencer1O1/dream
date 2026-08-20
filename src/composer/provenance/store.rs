use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::DreamError;

use super::super::output;

pub const STORE_REL: &str = ".dream/provenance.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Store {
    pub target: String,
    #[serde(default)]
    pub units: BTreeMap<String, UnitState>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitState {
    pub artifacts: Vec<String>,
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
            units: BTreeMap::new(),
        }
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
        if reserved(rel) {
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
        let mut artifacts: Vec<String> = artifacts.into_iter().collect();
        artifacts.sort();
        if artifacts.is_empty() {
            self.units.remove(unit);
        } else {
            self.units.insert(unit.to_string(), UnitState { artifacts });
        }
    }

    pub fn has_artifacts(&self) -> bool {
        self.units.values().any(|unit| !unit.artifacts.is_empty())
    }

    pub fn drop_owned(&self, dest: &Path) -> Result<(), DreamError> {
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
    fn owner_prefers_unit_then_unmanaged() {
        let mut store = Store::new("rust");
        store.set_artifacts(
            "server.foo",
            HashSet::from(["src/server.rs".into(), "src/routes.rs".into()]),
        );
        assert_eq!(
            store.owner("src/server.rs"),
            Owner::Unit("server.foo".into())
        );
        assert_eq!(store.owner("README.md"), Owner::Unmanaged);
        assert_eq!(store.owner(".dream/provenance.json"), Owner::Project);
    }
}
