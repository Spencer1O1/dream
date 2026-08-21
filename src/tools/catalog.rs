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
        self.tool_list()
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
        assert!(catalog.contains("- list_files:"));
        assert!(catalog.contains("- read_file:"));
        assert!(catalog.contains("- write_file:"));
        assert!(catalog.contains("- http_request:"));
        assert!(catalog.contains("- dream_error:"));
        assert!(!catalog.contains("contiguous"));
        assert!(!catalog.contains("--strict"));
        assert!(!catalog.contains("locked"));
        assert!(!catalog.contains("artifacts"));
    }

    #[test]
    fn composer_has_source_and_write_not_runtime() {
        let catalog = Registry::composer().prompt_catalog();
        assert!(catalog.contains("Composer"));
        assert!(catalog.contains("- write_file:"));
        assert!(catalog.contains("- read_file:"));
        assert!(catalog.contains("- remove_file:"));
        assert!(!catalog.contains("set_dependencies"));
        assert!(!catalog.contains("set_toolchain"));
        assert!(!catalog.contains("Runtime"));
        assert!(!catalog.contains("stdout"));
        assert!(catalog.contains("owned files"));
        assert!(!catalog.contains("artifacts"));
        let with_project =
            Registry::composer_for(Some(crate::toolchain::Toolchain::parse("cargo").unwrap()))
                .prompt_catalog();
        assert!(!with_project.contains("set_dependencies"));
        for name in ["write_file", "remove_file", "read_file"] {
            let description = with_project
                .lines()
                .find(|line| line.contains(&format!("- {name}:")))
                .unwrap();
            assert!(
                !description.contains("locked"),
                "{name} description repeats the lock rule: {description}"
            );
        }
    }

    #[test]
    fn repair_has_write_without_unit_or_deps() {
        let registry = Registry::repair();
        let catalog = registry.prompt_catalog();
        assert!(catalog.contains("- write_file:"));
        assert!(catalog.contains("- read_file:"));
        assert!(!catalog.contains("remove_file"));
        assert!(!catalog.contains("set_dependencies"));
        let write = registry
            .tools()
            .iter()
            .find(|tool| tool.spec().name == "write_file")
            .unwrap()
            .spec();
        assert!(write.parameters["properties"].get("unit").is_none());
        assert!(!write.description.contains("unit"));
    }

    #[test]
    fn toolchain_is_set_toolchain_only() {
        let catalog = Registry::toolchain().prompt_catalog();
        assert!(catalog.contains("- set_toolchain:"));
        assert!(!catalog.contains("write_file"));
        assert!(!catalog.contains("dream_error"));
    }
}
