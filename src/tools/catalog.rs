use std::fmt::Write;

use crate::flags::ActiveFlags;

use super::{Family, Registry, Tool};

impl Registry {
    pub fn instructions(&self, preamble: &str, flags: &ActiveFlags) -> String {
        let tools = self.prompt_catalog();
        match flags.prompt_catalog() {
            Some(catalog) => format!("{preamble}\n\n{tools}\n{catalog}"),
            None => format!("{preamble}\n\n{tools}"),
        }
    }

    pub fn prompt_catalog(&self) -> String {
        let mut out = self.tool_list();
        out.push_str(
            "These tools are the entire interface. Must use multiple tool calls in one turn for the next contiguous calls that can run without waiting on a result.\n",
        );
        out
    }

    pub(crate) fn tool_list(&self) -> String {
        let mut out = String::from("Tools:\n");
        for family in Family::ORDER {
            let members: Vec<&dyn Tool> = self
                .tools()
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
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpreter_lists_each_family() {
        let catalog = Registry::interpreter().prompt_catalog();
        assert!(catalog.contains("Source"));
        assert!(catalog.contains("Runtime"));
        assert!(!catalog.contains("Composer"));
        assert!(catalog.contains("Control"));
        assert!(catalog.contains("- list_source_files:"));
        assert!(catalog.contains("- stdout:"));
        assert!(catalog.contains("- dream_error:"));
        assert!(catalog.contains("next contiguous calls that can run without waiting on a result"));
        assert!(!catalog.contains("--strict"));
        assert!(!catalog.contains("locked"));
        assert!(!catalog.contains("artifacts"));
    }

    #[test]
    fn composer_has_source_and_write_not_runtime() {
        let catalog = Registry::composer().prompt_catalog();
        assert!(catalog.contains("Composer"));
        assert!(catalog.contains("- write_output_file:"));
        assert!(catalog.contains("- remove_output_file:"));
        assert!(!catalog.contains("set_dependencies"));
        assert!(!catalog.contains("set_builder"));
        assert!(!catalog.contains("Runtime"));
        assert!(!catalog.contains("stdout"));
        let with_project =
            Registry::composer_for(Some(crate::builder::Builder::parse("cargo").unwrap()))
                .prompt_catalog();
        assert!(with_project.contains("Project"));
        assert!(with_project.contains("- set_dependencies:"));
    }

    #[test]
    fn repair_has_write_without_unit_or_deps() {
        let registry = Registry::repair();
        let catalog = registry.prompt_catalog();
        assert!(catalog.contains("- write_output_file:"));
        assert!(catalog.contains("- remove_output_file:"));
        assert!(!catalog.contains("set_dependencies"));
        let write = registry
            .tools()
            .iter()
            .find(|tool| tool.spec().name == "write_output_file")
            .unwrap()
            .spec();
        assert!(write.parameters["properties"].get("unit").is_none());
        assert!(!write.description.contains("unit"));
    }

    #[test]
    fn builder_is_set_builder_only() {
        let catalog = Registry::builder().prompt_catalog();
        assert!(catalog.contains("- set_builder:"));
        assert!(!catalog.contains("write_output_file"));
        assert!(!catalog.contains("dream_error"));
    }
}
