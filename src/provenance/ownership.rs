use std::collections::HashSet;
use std::path::Path;

use crate::error::DreamError;
use crate::toolchain::ToolchainSpec;

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
    spec: Option<&ToolchainSpec>,
) -> Result<(), DreamError> {
    if reserved(rel) {
        return Err(DreamError::composer(
            "cannot write Dream-owned project metadata",
        ));
    }
    if spec.is_some_and(|spec| spec.is_wipe(rel)) {
        return Err(DreamError::composer(format!(
            "cannot write `{rel}`; wipe-only"
        )));
    }
    if spec.is_some_and(|spec| spec.is_setup(rel)) {
        return authorize_setup(store, dest, rel);
    }
    let owner = owner_including_job(store, rel, unit, this_job);
    reject_if_locked(store, unit, &owner)?;
    match owner {
        Owner::Project => Err(DreamError::composer(format!(
            "cannot write `{rel}`; wipe-only"
        ))),
        Owner::Unit(owner) => match unit {
            Some(unit) if owner == unit => Ok(()),
            None => Ok(()),
            Some(_) => Err(DreamError::composer(format!(
                "dest `{rel}` is owned by `{owner}`"
            ))),
        },
        Owner::Unmanaged => {
            if unit.is_none() {
                return Err(DreamError::composer(format!(
                    "repair cannot create `{rel}`"
                )));
            }
            if dest.join(rel).exists() {
                return Err(DreamError::composer(format!("dest `{rel}` is user-owned")));
            }
            Ok(())
        }
    }
}

fn authorize_setup(store: &Store, dest: &Path, rel: &str) -> Result<(), DreamError> {
    match store.owner(rel) {
        Owner::Unit(owner) => Err(DreamError::composer(format!(
            "dest `{rel}` is owned by `{owner}`"
        ))),
        Owner::Unmanaged if dest.join(rel).exists() => {
            Err(DreamError::composer(format!("dest `{rel}` is user-owned")))
        }
        Owner::Project | Owner::Unmanaged => Ok(()),
    }
}

pub fn authorize_read(
    store: &Store,
    dest: &Path,
    rel: &str,
    spec: Option<&ToolchainSpec>,
) -> Result<(), DreamError> {
    if reserved(rel) {
        return Err(DreamError::composer(
            "cannot read Dream-owned project metadata",
        ));
    }
    if spec.is_some_and(|spec| spec.is_wipe(rel)) {
        return Err(DreamError::composer(format!("cannot read `{rel}`")));
    }
    if spec.is_some_and(|spec| spec.is_setup(rel)) {
        if !dest.join(rel).is_file() {
            return Err(DreamError::composer(format!(
                "dest file `{rel}` does not exist"
            )));
        }
        return Ok(());
    }
    match store.owner(rel) {
        Owner::Unit(_) => {
            if !dest.join(rel).is_file() {
                return Err(DreamError::composer(format!(
                    "dest file `{rel}` does not exist"
                )));
            }
            Ok(())
        }
        Owner::Project => Err(DreamError::composer(format!("cannot read `{rel}`"))),
        Owner::Unmanaged => Err(DreamError::composer(format!("dest `{rel}` is user-owned"))),
    }
}

pub fn authorize_remove(
    store: &Store,
    rel: &str,
    unit: Option<&str>,
    this_job: Option<&HashSet<String>>,
    spec: Option<&ToolchainSpec>,
) -> Result<(), DreamError> {
    if reserved(rel) {
        return Err(DreamError::composer(
            "cannot remove Dream-owned project metadata",
        ));
    }
    if spec.is_some_and(|spec| spec.is_wipe(rel)) {
        return Err(DreamError::composer(format!(
            "cannot remove `{rel}`; wipe-only"
        )));
    }
    if spec.is_some_and(|spec| spec.is_setup(rel)) {
        return match store.owner(rel) {
            Owner::Unit(owner) => Err(DreamError::composer(format!(
                "dest `{rel}` is owned by `{owner}`"
            ))),
            Owner::Project | Owner::Unmanaged => Ok(()),
        };
    }
    let owner = owner_including_job(store, rel, unit, this_job);
    reject_if_locked(store, unit, &owner)?;
    match owner {
        Owner::Unit(owner) if unit == Some(owner.as_str()) => Ok(()),
        Owner::Unit(owner) => Err(DreamError::composer(format!(
            "dest `{rel}` is owned by `{owner}`"
        ))),
        Owner::Project => Err(DreamError::composer(format!(
            "cannot remove `{rel}`; wipe-only"
        ))),
        Owner::Unmanaged => Err(DreamError::composer(format!("dest `{rel}` is user-owned"))),
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

        let cargo = crate::toolchain::Toolchain::parse("cargo").unwrap().spec();
        authorize_write(
            &store,
            dest.path(),
            "src/main.rs",
            Some("main.foo"),
            None,
            cargo,
        )
        .unwrap();
        let err = authorize_write(
            &store,
            dest.path(),
            "src/main.rs",
            Some("other.foo"),
            None,
            cargo,
        )
        .unwrap_err();
        assert!(err.to_string().contains("owned by `main.foo`"));

        let unmanaged = authorize_write(
            &store,
            dest.path(),
            "README.md",
            Some("main.foo"),
            None,
            cargo,
        )
        .unwrap_err();
        assert!(unmanaged.to_string().contains("user-owned"));

        store.mark_project("Cargo.toml");
        store.mark_project("target");
        authorize_write(
            &store,
            dest.path(),
            "Cargo.toml",
            Some("main.foo"),
            None,
            cargo,
        )
        .unwrap();
        let wipe = authorize_write(&store, dest.path(), "target", Some("main.foo"), None, cargo)
            .unwrap_err();
        assert!(wipe.to_string().contains("wipe-only"));
        let child = authorize_write(
            &store,
            dest.path(),
            "target/foo.rs",
            Some("main.foo"),
            None,
            cargo,
        )
        .unwrap_err();
        assert!(child.to_string().contains("wipe-only"));

        let reserved_err = authorize_write(
            &store,
            dest.path(),
            STORE_REL,
            Some("main.foo"),
            None,
            cargo,
        )
        .unwrap_err();
        assert!(reserved_err.to_string().contains("project metadata"));

        let repair_new =
            authorize_write(&store, dest.path(), "src/new.rs", None, None, cargo).unwrap_err();
        assert!(repair_new.to_string().contains("repair cannot create"));
        authorize_write(&store, dest.path(), "src/main.rs", None, None, cargo).unwrap();

        store.set_lock("main.foo", "abc".into());
        authorize_unit(&store, "main.foo").unwrap_err();
        let locked = authorize_write(
            &store,
            dest.path(),
            "src/main.rs",
            Some("main.foo"),
            None,
            cargo,
        )
        .unwrap_err();
        assert!(locked.to_string().contains("`main.foo` is locked"));
        let repair_locked =
            authorize_write(&store, dest.path(), "src/main.rs", None, None, cargo).unwrap_err();
        assert!(repair_locked.to_string().contains("`main.foo` is locked"));
    }
}
