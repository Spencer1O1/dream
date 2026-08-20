use std::path::Path;

use crate::builder::BuilderSpec;
use crate::error::DreamError;
use crate::provenance::Store;

use super::manifest;

pub fn reconcile(dest: &Path, spec: &BuilderSpec, store: &mut Store) -> Result<(), DreamError> {
    let wanted = store.union_dependencies()?;
    manifest::apply(dest, spec, &wanted, &mut store.installed)?;
    store.save(dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::Builder;
    use crate::project::init;
    use crate::provenance::Dependency;
    use std::fs;

    #[test]
    fn union_of_unit_dependencies_becomes_the_manifest() {
        let dest = tempfile::tempdir().unwrap();
        let spec = Builder::parse("cargo").unwrap().spec().unwrap();
        let mut store = Store::new("rust");
        init(dest.path(), spec, "demo", &mut store).unwrap();
        store.set_dependencies(
            "main.foo",
            vec![Dependency {
                name: "serde".into(),
                version: None,
                features: vec!["derive".into()],
            }],
        );
        store.set_dependencies(
            "utils.foo",
            vec![Dependency {
                name: "tokio".into(),
                version: None,
                features: vec![],
            }],
        );
        reconcile(dest.path(), spec, &mut store).unwrap();
        let text = fs::read_to_string(dest.path().join("Cargo.toml")).unwrap();
        assert!(text.contains("serde"));
        assert!(text.contains("tokio"));
        assert_eq!(store.installed, vec!["serde", "tokio"]);
    }
}
