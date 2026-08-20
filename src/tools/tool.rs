use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;

use serde_json::{json, Value};

use crate::builder::Builder;
use crate::composer::provenance::Store;
use crate::error::DreamError;
use crate::source::{DepGraph, Project};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Source,
    Runtime,
    Composer,
    Control,
}

impl Family {
    pub(crate) const ORDER: [Self; 4] =
        [Self::Source, Self::Runtime, Self::Composer, Self::Control];
}

impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Source => "Source",
            Self::Runtime => "Runtime",
            Self::Composer => "Composer",
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

pub enum WriteSlot<'a> {
    Compose {
        artifacts: &'a mut HashMap<String, HashSet<String>>,
        fresh: bool,
    },
    Repair,
}

pub struct ToolCtx<'a> {
    pub project: &'a Project,
    pub deps: &'a mut DepGraph,
    pub dest: Option<&'a Path>,
    pub store: Option<&'a Store>,
    pub write: Option<WriteSlot<'a>>,
    pub builder: Option<&'a mut Option<Builder>>,
}

pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError>;
}
