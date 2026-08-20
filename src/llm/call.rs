use serde_json::{json, Value};

use crate::error::DreamError;

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

impl FunctionCall {
    pub fn parsed_args(&self) -> Result<Value, DreamError> {
        if self.arguments.trim().is_empty() {
            Ok(json!({}))
        } else {
            serde_json::from_str(&self.arguments).map_err(|_| {
                DreamError::runtime(format!("invalid arguments for tool `{}`", self.name))
            })
        }
    }
}

#[derive(Debug)]
pub struct ResponseTurn {
    pub output: Vec<Value>,
    pub function_calls: Vec<FunctionCall>,
}
