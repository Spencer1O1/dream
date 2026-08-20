mod composer;
mod control;
mod runtime;
mod source;

use serde_json::{json, Map, Value};

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
    const ORDER: [Self; 4] = [Self::Source, Self::Runtime, Self::Composer, Self::Control];

    fn heading(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Runtime => "Runtime",
            Self::Composer => "Composer",
            Self::Control => "Control",
        }
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

pub struct ToolCtx<'a> {
    pub project: &'a Project,
    pub deps: &'a mut DepGraph,
}

pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError>;
}

type FamilyTools = fn() -> Vec<Box<dyn Tool>>;

pub struct Registry {
    tools: Vec<Box<dyn Tool>>,
}

impl Registry {
    pub fn interpreter() -> Self {
        Self::gather(&[source::tools, runtime::tools, control::tools])
    }

    pub fn composer() -> Self {
        Self::gather(&[source::tools, composer::tools, control::tools])
    }

    fn gather(families: &[FamilyTools]) -> Self {
        let mut tools = Vec::new();
        for family in families {
            tools.extend(family());
        }
        Self { tools }
    }

    #[cfg(test)]
    pub fn names(&self) -> Vec<&'static str> {
        self.tools.iter().map(|tool| tool.spec().name).collect()
    }

    pub fn schemas(&self) -> Vec<Value> {
        self.tools.iter().map(|tool| tool.spec().schema()).collect()
    }

    pub fn prompt_catalog(&self) -> String {
        let mut out = String::from("Tools:\n");
        for family in Family::ORDER {
            let members: Vec<&dyn Tool> = self
                .tools
                .iter()
                .map(Box::as_ref)
                .filter(|tool| tool.spec().family == family)
                .collect();
            if members.is_empty() {
                continue;
            }
            out.push('\n');
            out.push_str(family.heading());
            out.push('\n');
            for tool in members {
                let spec = tool.spec();
                out.push_str("- ");
                out.push_str(spec.name);
                out.push_str(": ");
                out.push_str(spec.description);
                out.push('\n');
            }
        }
        out
    }

    pub fn call(
        &self,
        name: &str,
        ctx: &mut ToolCtx<'_>,
        args: &Value,
    ) -> Result<String, DreamError> {
        let tool = self
            .tools
            .iter()
            .find(|tool| tool.spec().name == name)
            .ok_or_else(|| DreamError::new(format!("unknown tool `{name}`")))?;
        tool.call(ctx, args)
    }
}

fn object_params(fields: &[(&str, Value)], required: &[&str]) -> Value {
    let mut properties = Map::new();
    for (name, schema) in fields {
        properties.insert((*name).to_string(), schema.clone());
    }
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

fn string_arg(description: &str) -> Value {
    json!({
        "type": "string",
        "description": description
    })
}

fn arg_str<'a>(args: &'a Value, name: &str) -> &'a str {
    args[name].as_str().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpreter_catalog_lists_each_family() {
        let registry = Registry::interpreter();
        let catalog = registry.prompt_catalog();
        assert!(catalog.contains("Source"));
        assert!(catalog.contains("Runtime"));
        assert!(!catalog.contains("Composer"));
        assert!(catalog.contains("Control"));
        assert!(catalog.contains("- list_source_files:"));
        assert!(catalog.contains("- stdout:"));
        assert!(catalog.contains("- dream_error:"));
        assert_eq!(
            registry.names(),
            vec![
                "list_source_files",
                "read_source_file",
                "stdout",
                "stdin",
                "dream_error"
            ]
        );
    }

    #[test]
    fn composer_registry_has_source_not_runtime() {
        let registry = Registry::composer();
        let names = registry.names();
        assert!(names.contains(&"list_source_files"));
        assert!(names.contains(&"read_source_file"));
        assert!(names.contains(&"dream_error"));
        assert!(!names.contains(&"stdout"));
        assert!(!names.contains(&"stdin"));
        assert!(!registry.prompt_catalog().contains("Runtime"));
    }
}
