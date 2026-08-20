use std::collections::HashSet;
use std::path::Path;

use crate::error::DreamError;

use super::store::{reserved, Owner, Store};

pub fn authorize_unit(store: &Store, unit: &str) -> Result<(), DreamError> {
    if store.is_locked(unit) {
        return Err(DreamError::composer(format!("`{unit}` is locked")));
    }
    Ok(())
}

pub fn authorize_write(
    store: &Store,
    dest: &Path,
    rel: &str,
    unit: Option<&str>,
    this_job: Option<&HashSet<String>>,
) -> Result<(), DreamError> {
    if reserved(rel) {
        return Err(DreamError::composer(
            "cannot write Dream-owned project metadata",
        ));
    }
    let owner = owner_including_job(store, rel, unit, this_job);
    reject_if_locked(store, unit, &owner)?;
    match owner {
        Owner::Project => Err(DreamError::composer(format!(
            "cannot write `{rel}`; Dream owns the manifest."
        ))),
        Owner::Unit(owner) => match unit {
            Some(unit) if owner == unit => Ok(()),
            None => Ok(()),
            Some(_) => Err(DreamError::composer(format!(
                "output `{rel}` is owned by `{owner}`"
            ))),
        },
        Owner::Unmanaged => {
            if unit.is_none() {
                return Err(DreamError::composer(format!(
                    "repair cannot create `{rel}`"
                )));
            }
            if dest.join(rel).exists() {
                return Err(DreamError::composer(format!(
                    "output `{rel}` is user-owned"
                )));
            }
            Ok(())
        }
    }
}

pub fn authorize_remove(
    store: &Store,
    rel: &str,
    unit: Option<&str>,
    this_job: Option<&HashSet<String>>,
) -> Result<(), DreamError> {
    if reserved(rel) {
        return Err(DreamError::composer(
            "cannot remove Dream-owned project metadata",
        ));
    }
    let owner = owner_including_job(store, rel, unit, this_job);
    reject_if_locked(store, unit, &owner)?;
    match owner {
        Owner::Unit(owner) if unit == Some(owner.as_str()) => Ok(()),
        Owner::Unit(owner) => Err(DreamError::composer(format!(
            "output `{rel}` is owned by `{owner}`"
        ))),
        Owner::Project => Err(DreamError::composer(format!(
            "cannot remove `{rel}`; Dream owns the manifest."
        ))),
        Owner::Unmanaged => Err(DreamError::composer(format!(
            "output `{rel}` is user-owned"
        ))),
    }
}

fn reject_if_locked(store: &Store, unit: Option<&str>, owner: &Owner) -> Result<(), DreamError> {
    if let Some(unit) = unit {
        authorize_unit(store, unit)?;
    }
    if let Owner::Unit(owner) = owner {
        authorize_unit(store, owner)?;
    }
    Ok(())
}

fn owner_including_job(
    store: &Store,
    rel: &str,
    unit: Option<&str>,
    this_job: Option<&HashSet<String>>,
) -> Owner {
    if let (Some(unit), Some(artifacts)) = (unit, this_job) {
        if artifacts.contains(rel) {
            return Owner::Unit(unit.to_string());
        }
    }
    store.owner(rel)
}

#[cfg(test)]
mod tests {
    use super::super::store::STORE_REL;
    use super::*;

    #[test]
    fn write_policy() {
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts(
            "main.foo",
            HashSet::from(["src/main.rs".into(), "src/old.rs".into()]),
        );
        std::fs::create_dir_all(dest.path().join("src")).unwrap();
        std::fs::write(dest.path().join("src/main.rs"), "old").unwrap();
        std::fs::write(dest.path().join("README.md"), "keep").unwrap();

        authorize_write(&store, dest.path(), "src/main.rs", Some("main.foo"), None).unwrap();
        let err = authorize_write(&store, dest.path(), "src/main.rs", Some("other.foo"), None)
            .unwrap_err();
        assert!(err.to_string().contains("owned by `main.foo`"));

        let unmanaged =
            authorize_write(&store, dest.path(), "README.md", Some("main.foo"), None).unwrap_err();
        assert!(unmanaged.to_string().contains("user-owned"));

        store.mark_project("Cargo.toml");
        let manifest =
            authorize_write(&store, dest.path(), "Cargo.toml", Some("main.foo"), None).unwrap_err();
        assert!(manifest.to_string().contains("Dream owns the manifest"));
        assert!(manifest.to_string().contains("Cargo.toml"));
        assert!(!manifest.to_string().contains("set_dependencies"));

        let reserved_err =
            authorize_write(&store, dest.path(), STORE_REL, Some("main.foo"), None).unwrap_err();
        assert!(reserved_err.to_string().contains("project metadata"));

        let repair_new =
            authorize_write(&store, dest.path(), "src/new.rs", None, None).unwrap_err();
        assert!(repair_new.to_string().contains("repair cannot create"));
        authorize_write(&store, dest.path(), "src/main.rs", None, None).unwrap();

        store.set_lock("main.foo", "abc".into());
        authorize_unit(&store, "main.foo").unwrap_err();
        let locked = authorize_write(&store, dest.path(), "src/main.rs", Some("main.foo"), None)
            .unwrap_err();
        assert!(locked.to_string().contains("`main.foo` is locked"));
        let repair_locked =
            authorize_write(&store, dest.path(), "src/main.rs", None, None).unwrap_err();
        assert!(repair_locked.to_string().contains("`main.foo` is locked"));
    }
}
