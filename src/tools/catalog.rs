//! Staple a preamble to the tool list and active flags.
//!
//! No Dream law of its own.

use std::fmt::Write;

use crate::flags::ActiveFlags;

use super::{Family, Registry, Tool};

impl Registry {
    pub fn instructions(&self, preamble: &str, flags: &ActiveFlags) -> String {
        let flags = flags.prompt_catalog().unwrap_or_default();
        let tools = self.tool_list();
        crate::prompt::paragraphs(&[preamble, &flags, &tools])
    }

    pub(crate) fn tool_list(&self) -> String {
        let mut sections = Vec::new();
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
            let mut section = family.to_string();
            for tool in members {
                let spec = tool.spec();
                write!(section, "\n- {}: {}", spec.name, spec.description)
                    .expect("write to String");
            }
            sections.push(section);
        }
        format!("Tools:\n\n{}", sections.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpreter_lists_runtime_not_composer() {
        let names = Registry::interpreter().names();
        assert!(names.contains(&"list_foo_files"));
        assert!(names.contains(&"read_foo_file"));
        assert!(names.contains(&"stdout"));
        assert!(names.contains(&"write_file"));
        assert!(!names.contains(&"write_source_file"));
        assert!(!names.contains(&"write_setup_file"));
        assert!(!names.iter().any(|name| name.contains("set_toolchain")));
    }

    #[test]
    fn composer_lists_source_and_dest_not_runtime() {
        let names = Registry::composer().names();
        assert!(names.contains(&"list_foo_files"));
        assert!(names.contains(&"read_foo_file"));
        assert!(names.contains(&"read_source_file"));
        assert!(names.contains(&"write_source_file"));
        assert!(names.contains(&"remove_source_file"));
        assert!(!names.contains(&"write_file"));
        assert!(!names.contains(&"read_file"));
        assert!(!names.contains(&"remove_file"));
        assert!(!names.contains(&"write_setup_file"));
        assert!(!names.contains(&"read_setup_file"));
        assert!(!names.contains(&"remove_setup_file"));
        assert!(!names.contains(&"stdout"));
        assert!(!names.contains(&"set_toolchain"));

        let cargo =
            Registry::composer_for(Some(crate::toolchain::Toolchain::parse("cargo").unwrap()))
                .names();
        assert!(cargo.contains(&"write_setup_file"));
        assert!(cargo.contains(&"read_setup_file"));
        assert!(cargo.contains(&"remove_setup_file"));
        let write = Registry::composer()
            .tools()
            .iter()
            .find(|tool| tool.spec().name == "write_source_file")
            .unwrap()
            .spec();
        assert!(write.parameters["properties"].get("unit").is_some());
        assert!(!write.description.contains("unit"));
    }

    #[test]
    fn toolchain_is_set_toolchain_only() {
        assert_eq!(Registry::toolchain().names(), vec!["set_toolchain"]);
    }

    #[test]
    fn instructions_are_law_then_flags_then_tools() {
        let registry = Registry::composer();
        let dump = registry.instructions("law", &crate::flags::ActiveFlags::new(true));
        assert!(dump.find("law").unwrap() < dump.find("Running with flags:").unwrap());
        assert!(dump.find("Running with flags:").unwrap() < dump.find("Tools:").unwrap());
        assert!(!dump.contains("\n\n\n"));
        assert!(!registry
            .instructions("law", &crate::flags::ActiveFlags::new(false))
            .contains("\n\n\n"));
    }
}
