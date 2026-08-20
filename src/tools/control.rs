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
            description: "Abort with a DreamError. Use when the program cannot be executed with these tools, or when --strict and meaning is importantly ambiguous.",
            parameters: object_params(
                &[(
                    "message",
                    string_arg("Error message without a DreamError prefix"),
                )],
                &["message"],
            ),
        }
    }

    fn call(&self, _ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let message = arg_str(args, "message");
        let message = if message.is_empty() {
            "unspecified error"
        } else {
            message
        };
        Err(DreamError::new(message))
    }
}
