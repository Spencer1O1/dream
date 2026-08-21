use std::fmt;

use serde_json::{json, Value};

use crate::error::DreamError;

use super::ctx::ToolCtx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Foo,
    Runtime,
    Composer,
    Project,
    Control,
}

impl Family {
    pub(crate) const ORDER: [Self; 5] = [
        Self::Foo,
        Self::Runtime,
        Self::Composer,
        Self::Project,
        Self::Control,
    ];
}

impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Foo => "Foo",
            Self::Runtime => "Runtime",
            Self::Composer => "Composer",
            Self::Project => "Project",
            Self::Control => "Control",
        })
    }
}

pub struct ToolSpec {
    pub name: &'static str,
    pub family: Family,
    pub description: &'static str,
    pub parameters: Value,
}

impl ToolSpec {
    pub fn schema(&self) -> Value {
        json!({
            "type": "function",
            "name": self.name,
            "description": self.description,
            "parameters": self.parameters,
            "strict": true
        })
    }
}

pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError>;
}
