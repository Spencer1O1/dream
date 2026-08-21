use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use crate::error::DreamError;
use crate::source::Project;

use super::lock::source_digest;
use super::store::{Store, UnitState};

pub fn report(
    dest: &Path,
    target: &str,
    project: &Project,
    unit: Option<&str>,
) -> Result<String, DreamError> {
    let store = super::open::require_store(dest, target)?;
    require_same_root(&store, project.root())?;
    match unit {
        Some(unit) => unit_report(&store, dest, project, unit),
        None => project_report(&store, dest, project),
    }
}

fn require_same_root(store: &Store, root: &Path) -> Result<(), DreamError> {
    let Some(prev) = store.source_root.as_deref() else {
        return Ok(());
    };
    let root = root.canonicalize()?;
    if prev != root.to_string_lossy().as_ref() {
        return Err(DreamError::usage("output is for another Dream project"));
    }
    Ok(())
}

fn project_report(store: &Store, dest: &Path, project: &Project) -> Result<String, DreamError> {
    let mut out = String::new();
    writeln!(out, "toolchain: {}", store.toolchain).expect("write");
    if let Some(name) = project.name() {
        writeln!(out, "name: {name}").expect("write");
    }
    if let Some(entry) = project.entry() {
        writeln!(out, "entry: {entry}").expect("write");
    }
    writeln!(out, "project:").expect("write");
    if store.project.is_empty() {
        writeln!(out, "  none").expect("write");
    } else {
        for path in &store.project {
            if store.is_locked(path) {
                writeln!(out, "  {path} locked").expect("write");
            } else {
                writeln!(out, "  {path}").expect("write");
            }
        }
    }
    writeln!(out, "units:").expect("write");
    let mut units: BTreeSet<String> = project.list_foo_files()?.into_iter().collect();
    units.extend(store.units.keys().cloned());
    if units.is_empty() {
        writeln!(out, "  none").expect("write");
    }
    for unit in units {
        write_unit(&mut out, store, dest, project, &unit, 1)?;
    }
    Ok(out)
}

fn unit_report(
    store: &Store,
    dest: &Path,
    project: &Project,
    unit: &str,
) -> Result<String, DreamError> {
    let mut out = String::new();
    write_unit(&mut out, store, dest, project, unit, 0)?;
    Ok(out)
}

fn write_unit(
    out: &mut String,
    store: &Store,
    dest: &Path,
    project: &Project,
    unit: &str,
    indent: usize,
) -> Result<(), DreamError> {
    let pad = "  ".repeat(indent);
    let inner = "  ".repeat(indent + 1);
    writeln!(out, "{pad}{unit}").expect("write");
    let state = store.units.get(unit);
    let source = project.read_foo_file(unit).ok();
    let status = status(
        state,
        source.as_ref().map(|unit| unit.source.as_str()),
        dest,
    );
    writeln!(out, "{inner}status: {status}").expect("write");
    if let Some(state) = state {
        if state.locked {
            let source_line = match source.as_ref() {
                None => "missing",
                Some(unit)
                    if state.source_hash.as_deref()
                        == Some(source_digest(&unit.source).as_str()) =>
                {
                    "matches"
                }
                Some(_) => "changed",
            };
            writeln!(out, "{inner}source: {source_line}").expect("write");
        }
        if state.artifacts.is_empty() {
            writeln!(out, "{inner}owned: none").expect("write");
        } else {
            writeln!(out, "{inner}owned:").expect("write");
            for path in &state.artifacts {
                writeln!(out, "{inner}  {path}").expect("write");
            }
        }
    } else {
        writeln!(out, "{inner}owned: none").expect("write");
    }
    Ok(())
}

fn status(state: Option<&UnitState>, source: Option<&str>, dest: &Path) -> &'static str {
    match (state, source) {
        (None, _) => "missing",
        (Some(_), None) => "invalid",
        (Some(state), Some(source)) if state.locked => {
            if state.artifacts.iter().any(|rel| !dest.join(rel).is_file()) {
                "invalid"
            } else if state.source_hash.as_deref() == Some(source_digest(source).as_str()) {
                "locked"
            } else {
                "stale"
            }
        }
        (Some(_), Some(_)) => "unlocked",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::Store;
    use crate::source::Project;
    use std::collections::HashSet;
    use std::fs;

    fn composed(root: &Path, unit: &str, rel: &str) -> tempfile::TempDir {
        let dest = tempfile::tempdir().unwrap();
        fs::create_dir_all(dest.path().join("src")).unwrap();
        fs::write(dest.path().join(rel), "fn main() {}").unwrap();
        let mut store = Store::new("cargo");
        store.set_source_root(root).unwrap();
        store.set_artifacts(unit, HashSet::from([rel.to_string()]));
        store.mark_project("Cargo.toml");
        store.save(dest.path()).unwrap();
        dest
    }

    #[test]
    fn unit_inspect_shows_unlocked_owned_files() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("main.foo"), "print hi").unwrap();
        let dest = composed(src.path(), "main.foo", "src/main.rs");
        let (project, unit) = Project::from_entry(&src.path().join("main.foo")).unwrap();
        let out = report(dest.path(), "rust", &project, Some(&unit.rel)).unwrap();
        assert!(out.starts_with("main.foo\n"));
        assert!(out.contains("status: unlocked"));
        assert!(out.contains("owned:\n    src/main.rs"));
        assert!(!out.contains("dependencies"));
        assert!(!out.contains("source:"));
    }

    #[test]
    fn locked_unit_reports_match_and_stale() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("main.foo"), "print hi").unwrap();
        let dest = composed(src.path(), "main.foo", "src/main.rs");
        crate::provenance::lock(dest.path(), "rust", &src.path().join("main.foo")).unwrap();
        let (project, _) = Project::from_entry(&src.path().join("main.foo")).unwrap();
        let out = report(dest.path(), "rust", &project, Some("main.foo")).unwrap();
        assert!(out.contains("status: locked"));
        assert!(out.contains("source: matches"));

        fs::write(src.path().join("main.foo"), "print bye").unwrap();
        let (project, _) = Project::from_entry(&src.path().join("main.foo")).unwrap();
        let out = report(dest.path(), "rust", &project, Some("main.foo")).unwrap();
        assert!(out.contains("status: stale"));
        assert!(out.contains("source: changed"));
    }

    #[test]
    fn project_inspect_lists_units_and_toml_name() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("main.foo"), "entry").unwrap();
        fs::write(src.path().join("utils.foo"), "utils").unwrap();
        fs::write(
            src.path().join("dream.toml"),
            "[project]\nname = \"demo\"\nentry = \"main.foo\"\n",
        )
        .unwrap();
        let dest = composed(src.path(), "main.foo", "src/main.rs");
        let project = Project::from_root(src.path()).unwrap();
        let out = report(dest.path(), "rust", &project, None).unwrap();
        assert!(out.contains("toolchain: cargo"));
        assert!(out.contains("name: demo"));
        assert!(out.contains("entry: main.foo"));
        assert!(out.contains("project:\n  Cargo.toml"));
        assert!(out.contains("  main.foo\n    status: unlocked"));
        assert!(out.contains("  utils.foo\n    status: missing"));
    }

    #[test]
    fn missing_store_is_usage() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("main.foo"), "entry").unwrap();
        let dest = tempfile::tempdir().unwrap();
        let (project, _) = Project::from_entry(&src.path().join("main.foo")).unwrap();
        let err = report(dest.path(), "rust", &project, Some("main.foo")).unwrap_err();
        assert!(err.to_string().contains("compose first"));
    }
}
