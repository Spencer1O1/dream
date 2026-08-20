use std::path::Path;

use crate::error::DreamError;
use crate::provenance::Store;
use crate::toolchain::ToolchainSpec;

use super::manifest;

pub fn init(
    dest: &Path,
    spec: &ToolchainSpec,
    package: &str,
    store: &mut Store,
) -> Result<(), DreamError> {
    let rel = manifest::path(spec)?;
    store.mark_project(rel);
    manifest::create_if_missing(dest, spec, package)?;
    store.save(dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolchain::Toolchain;
    use std::collections::HashSet;
    use std::fs;

    fn cargo() -> &'static ToolchainSpec {
        Toolchain::parse("cargo").unwrap().spec().unwrap()
    }

    #[test]
    fn writes_package_name_and_marks_project() {
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("rust");
        init(dest.path(), cargo(), "multifile", &mut store).unwrap();
        let text = fs::read_to_string(dest.path().join("Cargo.toml")).unwrap();
        assert!(text.contains("name = \"multifile\""));
        assert_eq!(store.project, vec!["Cargo.toml"]);
        assert_eq!(
            store.owner("Cargo.toml"),
            crate::provenance::store::Owner::Project
        );
    }

    #[test]
    fn steals_an_existing_unit_owned_manifest() {
        let dest = tempfile::tempdir().unwrap();
        fs::write(
            dest.path().join("Cargo.toml"),
            "[package]\nname = \"already\"\nversion = \"0.2.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts(
            "main.foo",
            HashSet::from(["Cargo.toml".into(), "src/main.rs".into()]),
        );
        init(dest.path(), cargo(), "multifile", &mut store).unwrap();
        let text = fs::read_to_string(dest.path().join("Cargo.toml")).unwrap();
        assert!(text.contains("name = \"already\""));
        assert!(!text.contains("multifile"));
        assert_eq!(store.units["main.foo"].artifacts, vec!["src/main.rs"]);
        assert_eq!(
            store.owner("Cargo.toml"),
            crate::provenance::store::Owner::Project
        );
    }
}
