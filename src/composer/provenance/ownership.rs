use std::collections::HashSet;
use std::path::Path;

use crate::error::DreamError;

use super::store::{reserved, Owner, Store};

pub fn authorize_write(
    store: &Store,
    dest: &Path,
    rel: &str,
    unit: Option<&str>,
    fresh: bool,
    this_job: Option<&HashSet<String>>,
) -> Result<(), DreamError> {
    if reserved(rel) {
        return Err(DreamError::runtime(
            "cannot write Dream-owned project metadata",
        ));
    }
    match owner_including_job(store, rel, unit, this_job) {
        Owner::Project => Err(DreamError::runtime(
            "cannot write Dream-owned project metadata",
        )),
        Owner::Unit(owner) => match unit {
            Some(unit) if owner == unit => Ok(()),
            None => Ok(()),
            Some(_) => Err(DreamError::runtime(format!(
                "output `{rel}` is owned by `{owner}`"
            ))),
        },
        Owner::Unmanaged => {
            let exists = dest.join(rel).exists();
            if unit.is_none() {
                return Err(DreamError::runtime(format!("repair cannot create `{rel}`")));
            }
            if exists && !fresh {
                return Err(DreamError::runtime(format!("output `{rel}` is user-owned")));
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
        return Err(DreamError::runtime(
            "cannot remove Dream-owned project metadata",
        ));
    }
    match owner_including_job(store, rel, unit, this_job) {
        Owner::Unit(owner) if unit == Some(owner.as_str()) => Ok(()),
        Owner::Unit(owner) => Err(DreamError::runtime(format!(
            "output `{rel}` is owned by `{owner}`"
        ))),
        Owner::Project => Err(DreamError::runtime(
            "cannot remove Dream-owned project metadata",
        )),
        Owner::Unmanaged => Err(DreamError::runtime(format!("output `{rel}` is user-owned"))),
    }
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

        authorize_write(
            &store,
            dest.path(),
            "src/main.rs",
            Some("main.foo"),
            false,
            None,
        )
        .unwrap();
        let err = authorize_write(
            &store,
            dest.path(),
            "src/main.rs",
            Some("other.foo"),
            false,
            None,
        )
        .unwrap_err();
        assert!(err.to_string().contains("owned by `main.foo`"));

        let unmanaged = authorize_write(
            &store,
            dest.path(),
            "README.md",
            Some("main.foo"),
            false,
            None,
        )
        .unwrap_err();
        assert!(unmanaged.to_string().contains("user-owned"));
        authorize_write(
            &store,
            dest.path(),
            "README.md",
            Some("main.foo"),
            true,
            None,
        )
        .unwrap();

        let reserved_err =
            authorize_write(&store, dest.path(), STORE_REL, Some("main.foo"), true, None)
                .unwrap_err();
        assert!(reserved_err.to_string().contains("project metadata"));

        let repair_new =
            authorize_write(&store, dest.path(), "src/new.rs", None, false, None).unwrap_err();
        assert!(repair_new.to_string().contains("repair cannot create"));
        authorize_write(&store, dest.path(), "src/main.rs", None, false, None).unwrap();
    }
}
