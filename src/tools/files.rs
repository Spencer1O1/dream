//! Catalog tool. Description says what it does; parameters say what to write. No Dream law.

use serde_json::{json, Value};

use crate::error::DreamError;

use crate::tools::Mode;

use super::reply;
use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(ListFiles), Box::new(ReadFile), Box::new(WriteFile)]
}

struct ListFiles;

impl Tool for ListFiles {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_files",
            family: Family::Runtime,
            description: "List every data file under the project. Returns each project-relative path. Not `.foo` files.",
            parameters: object_params(&[], &[]),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, _args: &Value) -> Result<String, DreamError> {
        require_lucid(&ctx.mode)?;
        Ok(json!({ "files": ctx.project.list_data_files()? }).to_string())
    }
}

struct ReadFile;

impl Tool for ReadFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_file",
            family: Family::Runtime,
            description: "Read one data file. Returns the project-relative path and the contents.",
            parameters: object_params(
                &[(
                    "path",
                    string_arg("Project-relative path of the data file to read"),
                )],
                &["path"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        require_lucid(&ctx.mode)?;
        let (path, contents) = match ctx.project.read_data_file(arg_str(args, "path")) {
            Ok(read) => read,
            Err(err) => return Ok(reply::refused(err)),
        };
        Ok(json!({ "path": path, "contents": contents }).to_string())
    }
}

struct WriteFile;

impl Tool for WriteFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write_file",
            family: Family::Runtime,
            description: "Write one data file. Replaces the whole file.",
            parameters: object_params(
                &[
                    (
                        "path",
                        string_arg("Project-relative path of the data file to write"),
                    ),
                    ("contents", string_arg("Exact text to write")),
                ],
                &["path", "contents"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        require_lucid(&ctx.mode)?;
        let path = ctx
            .project
            .write_data_file(arg_str(args, "path"), arg_str(args, "contents"))?;
        Ok(json!({ "ok": true, "path": path }).to_string())
    }
}

fn require_lucid(mode: &Mode<'_>) -> Result<(), DreamError> {
    if matches!(mode, Mode::Lucid) {
        Ok(())
    } else {
        Err(DreamError::runtime(
            "data files are only available when interpreting",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{DepGraph, Project};
    use crate::tools::ToolCtx;
    use serde_json::json;
    use std::fs;

    fn lucid_ctx<'a>(project: &'a Project, deps: &'a mut DepGraph) -> ToolCtx<'a> {
        ToolCtx::lucid(project, deps)
    }

    #[test]
    fn lists_reads_and_writes_data() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.foo"), "entry").unwrap();
        fs::write(dir.path().join("users.json"), "[1]").unwrap();
        let (project, unit) = Project::from_entry(&dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let mut ctx = lucid_ctx(&project, &mut deps);
        let listed: Value =
            serde_json::from_str(&ListFiles.call(&mut ctx, &json!({})).unwrap()).unwrap();
        assert_eq!(listed["files"], json!(["users.json"]));
        let read: Value = serde_json::from_str(
            &ReadFile
                .call(&mut ctx, &json!({"path": "users.json"}))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(read["contents"], "[1]");
        WriteFile
            .call(&mut ctx, &json!({"path": "out/note.txt", "contents": "hi"}))
            .unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("out/note.txt")).unwrap(),
            "hi"
        );
        let foo = ReadFile
            .call(&mut ctx, &json!({"path": "main.foo"}))
            .unwrap();
        assert!(crate::tools::reply::warning_of(&foo)
            .unwrap()
            .contains("read_foo_file"));
    }
}
