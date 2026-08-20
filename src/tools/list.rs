use serde_json::{json, Value};

use crate::error::DreamError;

use super::{object_params, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct ListSourceFiles;

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
