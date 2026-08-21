use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::Value;

use super::session::Session;
use crate::error::DreamError;
use crate::provenance::{self, Store};
use crate::source::DepGraph;
use crate::toolchain::Toolchain;

pub(crate) struct ComposeState {
    pub dest: std::path::PathBuf,
    pub store: Store,
    pub fresh: bool,
}

impl ComposeState {
    pub fn open(dest: &Path, target: &str, fresh: bool) -> Result<Self, DreamError> {
        let (store, fresh) = provenance::open(dest, target, fresh)?;
        Ok(Self {
            dest: dest.to_path_buf(),
            store,
            fresh,
        })
    }

    pub async fn compose(
        &mut self,
        session: &Session<'_>,
        deps: &mut DepGraph,
        input: &mut Vec<Value>,
        toolchain: Option<Toolchain>,
    ) -> Result<(), DreamError> {
        let mut artifacts = HashMap::new();
        crate::trace::job("compose", session.instructions, input);
        session
            .write_until_settled(
                self,
                deps,
                input,
                super::session::WriteLoop {
                    artifacts: &mut artifacts,
                    repair: false,
                    toolchain,
                    registry: session.registry,
                    instructions: session.instructions,
                    schemas: session.schemas,
                },
            )
            .await?;
        provenance::require_composed(&artifacts, &self.store, &deps.reached_units())?;
        self.settle(artifacts, toolchain)
    }

    fn settle(
        &mut self,
        artifacts: HashMap<String, HashSet<String>>,
        toolchain: Option<Toolchain>,
    ) -> Result<(), DreamError> {
        for (unit, paths) in artifacts {
            provenance::reconcile(&mut self.store, &self.dest, &unit, paths)?;
        }
        let _ = toolchain;
        self.store.save(&self.dest)?;
        Ok(())
    }

    pub(crate) fn merge_writes(
        &mut self,
        artifacts: HashMap<String, HashSet<String>>,
    ) -> Result<(), DreamError> {
        for (unit, paths) in artifacts {
            let mut merged: HashSet<String> = self
                .store
                .units
                .get(&unit)
                .map(|state| state.artifacts.iter().cloned().collect())
                .unwrap_or_default();
            merged.extend(paths);
            merged.retain(|rel| self.dest.join(rel).is_file());
            self.store.set_artifacts(&unit, merged);
        }
        self.store.save(&self.dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn settle_reconciles_each_unit_that_wrote() {
        let dest = tempfile::tempdir().unwrap();
        let mut state = ComposeState::open(dest.path(), "rust", true).unwrap();
        fs::create_dir_all(dest.path().join("src")).unwrap();
        fs::write(dest.path().join("src/old.rs"), "gone").unwrap();
        state
            .store
            .set_artifacts("main.foo", HashSet::from(["src/old.rs".into()]));

        state
            .settle(
                HashMap::from([
                    ("main.foo".into(), HashSet::from(["src/main.rs".into()])),
                    ("utils.foo".into(), HashSet::from(["src/lib.rs".into()])),
                ]),
                None,
            )
            .unwrap();

        assert!(!dest.path().join("src/old.rs").exists());
        assert_eq!(state.store.units["main.foo"].artifacts, vec!["src/main.rs"]);
        assert_eq!(state.store.units["utils.foo"].artifacts, vec!["src/lib.rs"]);
    }

    #[test]
    fn merge_writes_keeps_existing_and_adds_new() {
        let dest = tempfile::tempdir().unwrap();
        let mut state = ComposeState::open(dest.path(), "rust", true).unwrap();
        fs::create_dir_all(dest.path().join("src")).unwrap();
        fs::write(dest.path().join("src/main.rs"), "main").unwrap();
        fs::write(dest.path().join("src/gone.rs"), "gone").unwrap();
        state.store.set_artifacts(
            "main.foo",
            HashSet::from(["src/main.rs".into(), "src/gone.rs".into()]),
        );
        fs::remove_file(dest.path().join("src/gone.rs")).unwrap();
        fs::write(dest.path().join("src/extra.rs"), "extra").unwrap();

        state
            .merge_writes(HashMap::from([(
                "main.foo".into(),
                HashSet::from(["src/extra.rs".into()]),
            )]))
            .unwrap();

        assert_eq!(
            state.store.units["main.foo"].artifacts,
            vec!["src/extra.rs", "src/main.rs"]
        );
        assert!(dest.path().join("src/main.rs").exists());
        assert!(!dest.path().join("src/gone.rs").exists());
    }
}
