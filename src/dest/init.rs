use std::fs;
use std::path::Path;

use crate::error::DreamError;
use crate::provenance::Store;
use crate::toolchain::ToolchainSpec;

pub fn init(dest: &Path, spec: &ToolchainSpec, store: &mut Store) -> Result<(), DreamError> {
    for rel in spec.owned_dest() {
        store.mark_project(&rel);
    }
    store.save(dest)?;
    Ok(())
}

/// Build/dep dirs (`target`, `bin`) exist for exec, not for compose.
pub fn ensure_output_dirs(dest: &Path, spec: &ToolchainSpec) -> Result<(), DreamError> {
    for rel in spec.project {
        let path = dest.join(rel);
        if path.is_file() {
            continue;
        }
        if Path::new(rel).extension().is_none() {
            fs::create_dir_all(&path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolchain::Toolchain;

    #[test]
    fn marks_setup_and_wipe_without_writing_the_manifest() {
        let dest = tempfile::tempdir().unwrap();
        let spec = Toolchain::parse("cargo").unwrap().spec().unwrap();
        let mut store = Store::new("cargo");
        init(dest.path(), spec, &mut store).unwrap();
        assert!(!dest.path().join("Cargo.toml").exists());
        assert!(!dest.path().join("target").exists());
        assert_eq!(
            store.owner("Cargo.toml"),
            crate::provenance::store::Owner::Project
        );
        assert_eq!(
            store.owner("target"),
            crate::provenance::store::Owner::Project
        );
        ensure_output_dirs(dest.path(), spec).unwrap();
        assert!(dest.path().join("target").is_dir());
        assert!(!dest.path().join("Cargo.toml").exists());
    }

    #[test]
    fn go_marks_mod_and_target() {
        let dest = tempfile::tempdir().unwrap();
        let spec = Toolchain::parse("go").unwrap().spec().unwrap();
        let mut store = Store::new("go");
        init(dest.path(), spec, &mut store).unwrap();
        assert_eq!(store.project, vec!["go.mod", "go.sum", "target"]);
        assert!(!dest.path().join("go.mod").exists());
        assert!(!dest.path().join("target").exists());
    }

    #[test]
    fn lua_has_no_setup() {
        let dest = tempfile::tempdir().unwrap();
        let spec = Toolchain::parse("lua").unwrap().spec().unwrap();
        let mut store = Store::new("lua");
        init(dest.path(), spec, &mut store).unwrap();
        assert!(store.project.is_empty());
    }

    #[test]
    fn node_marks_package_json_without_writing_it() {
        let dest = tempfile::tempdir().unwrap();
        let spec = Toolchain::parse("node").unwrap().spec().unwrap();
        let mut store = Store::new("js");
        init(dest.path(), spec, &mut store).unwrap();
        assert!(!dest.path().join("package.json").exists());
        assert_eq!(
            store.project,
            vec!["node_modules", "package-lock.json", "package.json"]
        );
    }
}
