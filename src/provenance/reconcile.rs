use std::collections::HashSet;
use std::path::Path;

use crate::error::DreamError;

use super::store::Store;
use crate::output;

pub fn reconcile(
    store: &mut Store,
    dest: &Path,
    unit: &str,
    new_artifacts: HashSet<String>,
) -> Result<(), DreamError> {
    let previous = store
        .units
        .get(unit)
        .map(|state| state.artifacts.clone())
        .unwrap_or_default();
    for old in &previous {
        if !new_artifacts.contains(old) {
            let _ = output::remove_file(dest, old);
        }
    }
    store.set_artifacts(unit, new_artifacts);
    store.save(dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn reconcile_deletes_only_stale_owned_files() {
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts(
            "main.foo",
            HashSet::from(["src/main.rs".into(), "src/old.rs".into()]),
        );
        fs::create_dir_all(dest.path().join("src")).unwrap();
        fs::write(dest.path().join("src/main.rs"), "old").unwrap();
        fs::write(dest.path().join("src/old.rs"), "gone").unwrap();
        fs::write(dest.path().join("README.md"), "keep").unwrap();

        reconcile(
            &mut store,
            dest.path(),
            "main.foo",
            HashSet::from(["src/main.rs".into()]),
        )
        .unwrap();
        assert!(!dest.path().join("src/old.rs").exists());
        assert!(dest.path().join("README.md").exists());
        assert_eq!(store.units["main.foo"].artifacts, vec!["src/main.rs"]);
    }
}
