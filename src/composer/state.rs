use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::Value;

use crate::error::DreamError;
use crate::source::DepGraph;

use super::provenance::{self, Store};
use super::session::Session;

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
    ) -> Result<(), DreamError> {
        let mut artifacts = HashMap::new();
        session
            .write_until_settled(self, deps, input, &mut artifacts, false)
            .await?;
        self.settle(artifacts)
    }

    fn settle(&mut self, artifacts: HashMap<String, HashSet<String>>) -> Result<(), DreamError> {
        for (unit, paths) in artifacts {
            provenance::reconcile(&mut self.store, &self.dest, &unit, paths)?;
        }
        Ok(())
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
            .settle(HashMap::from([
                ("main.foo".into(), HashSet::from(["src/main.rs".into()])),
                ("utils.foo".into(), HashSet::from(["src/lib.rs".into()])),
            ]))
            .unwrap();

        assert!(!dest.path().join("src/old.rs").exists());
        assert_eq!(state.store.units["main.foo"].artifacts, vec!["src/main.rs"]);
        assert_eq!(state.store.units["utils.foo"].artifacts, vec!["src/lib.rs"]);
    }
}
