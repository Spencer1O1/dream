use serde_json::{json, Value};

use crate::error::DreamError;

use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(ListSourceFiles), Box::new(ReadSourceFile)]
}

struct ListSourceFiles;

impl Tool for ListSourceFiles {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_source_files",
            family: Family::Source,
            description: "List project-relative paths of every .foo file. Paths only, no contents. List instead of inventing filenames.",
            parameters: object_params(&[], &[]),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, _args: &Value) -> Result<String, DreamError> {
        let files = ctx.project.list_source_files()?;
        Ok(json!({ "files": files }).to_string())
    }
}

struct ReadSourceFile;

impl Tool for ReadSourceFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_source_file",
            family: Family::Source,
            description: "Read one .foo source unit inside the project. Do not invent source that is not in the project.",
            parameters: object_params(
                &[("path", string_arg("Project-relative .foo path"))],
                &["path"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let unit = ctx.project.read_source_file(arg_str(args, "path"))?;
        ctx.deps.record_read(&unit.rel)?;
        Ok(json!({ "path": unit.rel, "source": unit.source }).to_string())
    }
}
