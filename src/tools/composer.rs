use serde_json::{json, Value};

use crate::composer::output;
use crate::error::DreamError;

use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(WriteOutputFile), Box::new(RemoveOutputFile)]
}

struct WriteOutputFile;

impl Tool for WriteOutputFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write_output_file",
            family: Family::Composer,
            description: "Write one file into the target project. Path is relative to the output root. Overwrites if the file exists.",
            parameters: object_params(
                &[
                    ("path", string_arg("Output-relative file path")),
                    ("contents", string_arg("Exact file contents")),
                ],
                &["path", "contents"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let staging = ctx.staging.ok_or_else(|| {
            DreamError::runtime("write_output_file is only available while composing")
        })?;
        let path = output::write_file(staging, arg_str(args, "path"), arg_str(args, "contents"))?;
        Ok(json!({ "ok": true, "path": path }).to_string())
    }
}

struct RemoveOutputFile;

impl Tool for RemoveOutputFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "remove_output_file",
            family: Family::Composer,
            description:
                "Remove one file from the target project. Path is relative to the output root.",
            parameters: object_params(
                &[("path", string_arg("Output-relative file path"))],
                &["path"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let staging = ctx.staging.ok_or_else(|| {
            DreamError::runtime("remove_output_file is only available while composing")
        })?;
        let path = output::remove_file(staging, arg_str(args, "path"))?;
        Ok(json!({ "ok": true, "path": path }).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{DepGraph, Project};
    use crate::tools::ToolCtx;
    use serde_json::json;

    #[test]
    fn writes_into_staging() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(unit.rel);
        let staging = tempfile::tempdir().unwrap();
        let mut ctx = ToolCtx {
            project: &project,
            deps: &mut deps,
            staging: Some(staging.path()),
        };
        let out = WriteOutputFile
            .call(
                &mut ctx,
                &json!({ "path": "hello.txt", "contents": "hello" }),
            )
            .unwrap();
        assert!(out.contains("hello.txt"));
        assert_eq!(
            std::fs::read_to_string(staging.path().join("hello.txt")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn rejects_escape() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(unit.rel);
        let staging = tempfile::tempdir().unwrap();
        let mut ctx = ToolCtx {
            project: &project,
            deps: &mut deps,
            staging: Some(staging.path()),
        };
        let err = WriteOutputFile
            .call(&mut ctx, &json!({ "path": "../secret", "contents": "no" }))
            .unwrap_err();
        assert!(err.to_string().contains("output write escapes -o"));
    }

    #[test]
    fn removes_a_written_file() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(unit.rel);
        let staging = tempfile::tempdir().unwrap();
        let mut ctx = ToolCtx {
            project: &project,
            deps: &mut deps,
            staging: Some(staging.path()),
        };
        WriteOutputFile
            .call(&mut ctx, &json!({ "path": "oops.rs", "contents": "nope" }))
            .unwrap();
        let out = RemoveOutputFile
            .call(&mut ctx, &json!({ "path": "oops.rs" }))
            .unwrap();
        assert!(out.contains("oops.rs"));
        assert!(!staging.path().join("oops.rs").exists());
    }
}
