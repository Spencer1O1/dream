use std::fmt::Write;

use serde_json::Value;

use crate::error::DreamError;
use crate::flags::ActiveFlags;
use crate::llm::FunctionCall;

use super::{Family, Tool, ToolCtx};

type FamilyTools = fn() -> Vec<Box<dyn Tool>>;

pub struct Registry {
    tools: Vec<Box<dyn Tool>>,
}

impl Registry {
    pub fn interpreter() -> Self {
        Self::gather(&[
            super::source::tools,
            super::runtime::tools,
            super::control::tools,
        ])
    }

    pub fn composer() -> Self {
        Self::gather(&[
            super::source::tools,
            super::composer::tools,
            super::control::tools,
        ])
    }

    pub fn builder() -> Self {
        Self::gather(&[super::builder::tools, super::control::tools])
    }

    fn gather(families: &[FamilyTools]) -> Self {
        Self {
            tools: families.iter().flat_map(|family| family()).collect(),
        }
    }

    #[cfg(test)]
    pub fn names(&self) -> Vec<&'static str> {
        self.tools.iter().map(|tool| tool.spec().name).collect()
    }

    pub fn schemas(&self) -> Vec<Value> {
        self.tools.iter().map(|tool| tool.spec().schema()).collect()
    }

    pub fn instructions(&self, preamble: &str, flags: &ActiveFlags) -> String {
        let tools = self.prompt_catalog();
        match flags.prompt_catalog() {
            Some(catalog) => format!("{preamble}\n\n{tools}\n{catalog}"),
            None => format!("{preamble}\n\n{tools}"),
        }
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
            writeln!(out, "\n{family}").expect("write to String");
            for tool in members {
                let spec = tool.spec();
                writeln!(out, "- {}: {}", spec.name, spec.description).expect("write to String");
            }
        }
        out.push('\n');
        out.push_str(
            "These tools are the entire interface. Prefer multiple tool calls in one turn when later calls do not need earlier results. Anything else is invalid.\n",
        );
        out
    }

    pub fn dispatch(
        &self,
        ctx: &mut ToolCtx<'_>,
        call: &FunctionCall,
    ) -> Result<String, DreamError> {
        let args = call.parsed_args()?;
        let tool = self
            .tools
            .iter()
            .find(|tool| tool.spec().name == call.name)
            .ok_or_else(|| DreamError::runtime(format!("unknown tool `{}`", call.name)))?;
        tool.call(ctx, &args)
    }
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
        assert!(catalog.contains("Prefer multiple tool calls in one turn"));
        assert!(!catalog.contains("--strict"));
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
    fn composer_registry_has_source_and_write_not_runtime() {
        let registry = Registry::composer();
        let catalog = registry.prompt_catalog();
        assert_eq!(
            registry.names(),
            vec![
                "list_source_files",
                "read_source_file",
                "write_output_file",
                "remove_output_file",
                "dream_error"
            ]
        );
        assert!(catalog.contains("Composer"));
        assert!(catalog.contains("- write_output_file:"));
        assert!(catalog.contains("- remove_output_file:"));
        assert!(!catalog.contains("set_builder"));
        assert!(!catalog.contains("Runtime"));
        assert!(!catalog.contains("stdout"));
    }

    #[test]
    fn builder_registry_is_set_builder_only() {
        let registry = Registry::builder();
        assert_eq!(registry.names(), vec!["set_builder", "dream_error"]);
        assert!(registry.prompt_catalog().contains("- set_builder:"));
        assert!(!registry.prompt_catalog().contains("write_output_file"));
    }
}
