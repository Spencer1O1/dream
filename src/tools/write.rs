use serde_json::{json, Value};

use crate::composer::output;
use crate::composer::provenance;
use crate::error::DreamError;

use super::composer::{claim_unit, dest_rel};
use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec, WriteSlot};

pub(super) struct WriteOutputFile;

impl Tool for WriteOutputFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write_output_file",
            family: Family::Composer,
            description: "Write one source (code) file owned by a .foo unit. unit is the project-relative .foo path. Path is relative to the output root. Overwrites if the file exists.",
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
        let claimed = if matches!(ctx.write, Some(WriteSlot::Compose { .. })) {
            Some(claim_unit(ctx, arg_str(args, "unit"))?)
        } else {
            None
        };
        let (dest, store, rel) = dest_rel(ctx, arg_str(args, "path"))?;
        match &mut ctx.write {
            Some(WriteSlot::Compose {
                artifacts, fresh, ..
            }) => {
                let unit = claimed
                    .ok_or_else(|| DreamError::runtime("write_output_file requires unit"))?;
                provenance::authorize_write(
                    store,
                    dest,
                    &rel,
                    Some(&unit),
                    *fresh,
                    artifacts.get(&unit),
                )?;
                let path = output::write_file(dest, &rel, arg_str(args, "contents"))?;
                artifacts.entry(unit).or_default().insert(path.clone());
                Ok(json!({ "ok": true, "path": path }).to_string())
            }
            Some(WriteSlot::Repair) => {
                provenance::authorize_write(store, dest, &rel, None, false, None)?;
                let path = output::write_file(dest, &rel, arg_str(args, "contents"))?;
                Ok(json!({ "ok": true, "path": path }).to_string())
            }
            None => Err(DreamError::runtime(
                "write_output_file is only available while composing",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::provenance::Store;
    use crate::source::{DepGraph, Project};
    use crate::tools::composer::compose_ctx;
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
        let mut ctx = compose_ctx(
            &project,
            &mut deps,
            dest.path(),
            &store,
            &mut artifacts,
            &mut claims,
            false,
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
        let mut ctx = compose_ctx(
            &project,
            &mut deps,
            dest.path(),
            &store,
            &mut artifacts,
            &mut claims,
            false,
        );
        let err = WriteOutputFile
            .call(&mut ctx, &write_args("utils.foo", "src/lib.rs", "no"))
            .unwrap_err();
        assert!(err.to_string().contains("read that unit first"));
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
        let mut ctx = compose_ctx(
            &project,
            &mut deps,
            dest.path(),
            &store,
            &mut artifacts,
            &mut claims,
            false,
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
        let mut ctx = ToolCtx {
            project: &project,
            deps: &mut deps,
            dest: Some(dest.path()),
            store: Some(&store),
            write: Some(WriteSlot::Repair),
            builder: None,
            toolchain: None,
        };
        let err = WriteOutputFile
            .call(&mut ctx, &write_args(&unit.rel, "src/new.rs", "no"))
            .unwrap_err();
        assert!(err.to_string().contains("repair cannot create"));
    }
}
