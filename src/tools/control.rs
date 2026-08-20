use serde_json::Value;

use crate::error::DreamError;

use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(DreamErrorTool)]
}

struct DreamErrorTool;

impl Tool for DreamErrorTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "dream_error",
            family: Family::Control,
            description: "Abort. Report why the program cannot continue.",
            parameters: object_params(&[("error", string_arg("What went wrong"))], &["error"]),
        }
    }

    fn call(&self, _ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let error = arg_str(args, "error");
        let error = if error.is_empty() {
            "unspecified error"
        } else {
            error
        };
        Err(DreamError::interpreter(error))
    }
}
