use serde_json::{json, Value};

use crate::error::DreamError;
use crate::output;
use crate::provenance;
use crate::tools::{Compose, Mode};

use super::composer::{claim_unit, dest_rel};
use super::reply;
use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct WriteOutputFile;

impl Tool for WriteOutputFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write_output_file",
            family: Family::Composer,
            description: "Write one source (code) file owned by a .foo unit. unit is the project-relative .foo path. Path is relative to the output root. Overwrites if the file exists. Fails if that unit is locked.",
            parameters: object_params(
                &[
                    ("unit", string_arg("Project-relative .foo that owns this file")),
                    ("path", string_arg("Output-relative file path")),
                    ("contents", string_arg("Exact file contents")),
                ],
                &["unit", "path", "contents"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let claimed = if matches!(ctx.mode, Mode::Compose(_)) {
            match claim_unit(ctx, arg_str(args, "unit")) {
                Ok(unit) => Some(unit),
                Err(err) => return Ok(reply::refused(err)),
            }
        } else {
            None
        };
        match &mut ctx.mode {
            Mode::Compose(Compose {
                dest,
                store,
                artifacts,
                fresh,
                ..
            }) => {
                let unit = claimed
                    .ok_or_else(|| DreamError::runtime("write_output_file requires unit"))?;
                let rel = dest_rel(dest, arg_str(args, "path"))?;
                if let Err(err) = provenance::authorize_write(
                    store,
                    dest,
                    &rel,
                    Some(&unit),
                    *fresh,
                    artifacts.get(&unit),
                ) {
                    return Ok(reply::refused(err));
                }
                let path = output::write_file(dest, &rel, arg_str(args, "contents"))?;
                artifacts.entry(unit).or_default().insert(path.clone());
                Ok(json!({ "ok": true, "path": path }).to_string())
            }
            Mode::Repair(repair) => {
                let dest = repair.dest;
                let store = repair.store;
                let rel = dest_rel(dest, arg_str(args, "path"))?;
                if let Err(err) = provenance::authorize_write(store, dest, &rel, None, false, None)
                {
                    return Ok(reply::refused(err));
                }
                let path = output::write_file(dest, &rel, arg_str(args, "contents"))?;
                Ok(json!({ "ok": true, "path": path }).to_string())
            }
            Mode::Lucid | Mode::Pick(_) => Err(DreamError::runtime(
                "write_output_file is only available while composing",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::Store;
    use crate::source::{DepGraph, Project};
    use crate::tools::ToolCtx;
    use serde_json::json;
    use std::collections::HashMap;

    fn write_args(unit: &str, path: &str, contents: &str) -> Value {
        json!({ "unit": unit, "path": path, "contents": contents })
    }

    #[test]
    fn writes_into_dest() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let store = Store::new("rust");
        let mut artifacts = HashMap::new();
        let mut claims = HashMap::new();
        let mut ctx = ToolCtx::compose(
            &project,
            &mut deps,
            Compose {
                dest: dest.path(),
                store: &store,
                artifacts: &mut artifacts,
                dependencies: &mut claims,
                fresh: false,
                toolchain: None,
            },
        );
        let out = WriteOutputFile
            .call(&mut ctx, &write_args(&unit.rel, "hello.txt", "hello"))
            .unwrap();
        assert!(out.contains("hello.txt"));
        assert_eq!(
            std::fs::read_to_string(dest.path().join("hello.txt")).unwrap(),
            "hello"
        );
        assert!(artifacts[&unit.rel].contains("hello.txt"));
    }

    #[test]
    fn rejects_a_unit_that_was_not_read() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        std::fs::write(project_dir.path().join("utils.foo"), "fn").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let store = Store::new("rust");
        let mut artifacts = HashMap::new();
        let mut claims = HashMap::new();
        let mut ctx = ToolCtx::compose(
            &project,
            &mut deps,
            Compose {
                dest: dest.path(),
                store: &store,
                artifacts: &mut artifacts,
                dependencies: &mut claims,
                fresh: false,
                toolchain: None,
            },
        );
        let out = WriteOutputFile
            .call(&mut ctx, &write_args("utils.foo", "src/lib.rs", "no"))
            .unwrap();
        assert!(out.contains("read that unit first"));
        assert_eq!(
            reply::warning_of(&out).as_deref(),
            Some("cannot write for `utils.foo`; read that unit first")
        );
    }

    #[test]
    fn rejects_escape() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let store = Store::new("rust");
        let mut artifacts = HashMap::new();
        let mut claims = HashMap::new();
        let mut ctx = ToolCtx::compose(
            &project,
            &mut deps,
            Compose {
                dest: dest.path(),
                store: &store,
                artifacts: &mut artifacts,
                dependencies: &mut claims,
                fresh: false,
                toolchain: None,
            },
        );
        let err = WriteOutputFile
            .call(&mut ctx, &write_args(&unit.rel, "../secret", "no"))
            .unwrap_err();
        assert!(err.to_string().contains("output write escapes -o"));
    }

    #[test]
    fn repair_rejects_a_new_path() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let store = Store::new("rust");
        let mut ctx = ToolCtx::repair(&project, &mut deps, dest.path(), &store);
        let out = WriteOutputFile
            .call(&mut ctx, &write_args(&unit.rel, "src/new.rs", "no"))
            .unwrap();
        assert!(reply::warning_of(&out)
            .unwrap()
            .contains("repair cannot create"));
    }

    #[test]
    fn locked_unit_is_a_warning() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dest.path().join("src")).unwrap();
        std::fs::write(dest.path().join("src/main.rs"), "old").unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts(
            &unit.rel,
            std::collections::HashSet::from(["src/main.rs".into()]),
        );
        store.set_lock(&unit.rel, "abc".into());
        let mut artifacts = HashMap::new();
        let mut claims = HashMap::new();
        let mut ctx = ToolCtx::compose(
            &project,
            &mut deps,
            Compose {
                dest: dest.path(),
                store: &store,
                artifacts: &mut artifacts,
                dependencies: &mut claims,
                fresh: false,
                toolchain: None,
            },
        );
        let out = WriteOutputFile
            .call(&mut ctx, &write_args(&unit.rel, "src/main.rs", "new"))
            .unwrap();
        assert_eq!(
            reply::warning_of(&out).as_deref(),
            Some("`main.foo` is locked")
        );
        assert_eq!(
            std::fs::read_to_string(dest.path().join("src/main.rs")).unwrap(),
            "old"
        );
    }
}
