use serde_json::Value;

use crate::error::DreamError;
use crate::llm::FunctionCall;

use super::{Tool, ToolCtx};

type FamilyTools = fn() -> Vec<Box<dyn Tool>>;

pub struct Registry {
    tools: Vec<Box<dyn Tool>>,
}

impl Registry {
    pub fn interpreter() -> Self {
        Self::gather(&[
            super::source::lucid_tools,
            super::runtime::tools,
            super::control::tools,
        ])
    }

    #[cfg(test)]
    pub fn composer() -> Self {
        Self::composer_for(None)
    }

    pub fn composer_for(toolchain: Option<crate::toolchain::Toolchain>) -> Self {
        if toolchain.and_then(|known| known.spec()).is_some() {
            Self::gather(&[
                super::source::compose_tools,
                super::composer::tools,
                super::deps::tools,
                super::control::tools,
            ])
        } else {
            Self::gather(&[
                super::source::compose_tools,
                super::composer::tools,
                super::control::tools,
            ])
        }
    }

    pub fn toolchain() -> Self {
        Self::gather(&[super::toolchain::tools])
    }

    pub fn repair() -> Self {
        Self::gather(&[
            super::source::compose_tools,
            super::composer::repair_tools,
            super::control::tools,
        ])
    }

    fn gather(families: &[FamilyTools]) -> Self {
        Self {
            tools: families.iter().flat_map(|family| family()).collect(),
        }
    }

    pub(crate) fn tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }

    #[cfg(test)]
    pub fn names(&self) -> Vec<&'static str> {
        self.tools.iter().map(|tool| tool.spec().name).collect()
    }

    pub fn schemas(&self) -> Vec<Value> {
        self.tools.iter().map(|tool| tool.spec().schema()).collect()
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
    fn interpreter_names() {
        assert_eq!(
            Registry::interpreter().names(),
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
    fn composer_names() {
        assert_eq!(
            Registry::composer().names(),
            vec![
                "list_source_files",
                "read_source_file",
                "write_output_file",
                "remove_output_file",
                "dream_error"
            ]
        );
        assert_eq!(
            Registry::composer_for(Some(crate::toolchain::Toolchain::parse("cargo").unwrap()))
                .names(),
            vec![
                "list_source_files",
                "read_source_file",
                "write_output_file",
                "remove_output_file",
                "set_dependencies",
                "dream_error"
            ]
        );
    }

    #[test]
    fn toolchain_names() {
        assert_eq!(Registry::toolchain().names(), vec!["set_toolchain"]);
    }

    #[test]
    fn foo_is_a_file() {
        let cargo = crate::toolchain::Toolchain::parse("cargo").unwrap();
        for registry in [
            Registry::interpreter(),
            Registry::composer(),
            Registry::composer_for(Some(cargo)),
            Registry::toolchain(),
            Registry::repair(),
        ] {
            for tool in registry.tools() {
                let spec = tool.spec();
                assert!(
                    !spec.description.contains("artifact"),
                    "{} description says artifact: {}",
                    spec.name,
                    spec.description
                );
                assert_foo_wording(spec.name, spec.description, false);
                for text in schema_descriptions(&spec.parameters) {
                    assert_foo_wording(spec.name, text, true);
                }
            }
        }
    }

    fn assert_foo_wording(tool: &str, text: &str, parameter: bool) {
        assert!(
            !text.contains("`.foo` path"),
            "{tool} says `.foo` path: {text}"
        );
        if !text.contains("`.foo`") {
            return;
        }
        assert!(
            text.contains("`.foo` file"),
            "{tool} says `.foo` without file: {text}"
        );
        if parameter {
            assert!(
                text.contains("path"),
                "{tool} parameter names a `.foo` file without path: {text}"
            );
        }
    }

    fn schema_descriptions(schema: &Value) -> Vec<&str> {
        let mut texts = Vec::new();
        if let Some(description) = schema.get("description").and_then(Value::as_str) {
            texts.push(description);
        }
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            for value in properties.values() {
                texts.extend(schema_descriptions(value));
            }
        }
        if let Some(items) = schema.get("items") {
            texts.extend(schema_descriptions(items));
        }
        texts
    }

    #[test]
    fn description_does_not_explain_parameters() {
        let cargo = crate::toolchain::Toolchain::parse("cargo").unwrap();
        for registry in [
            Registry::interpreter(),
            Registry::composer(),
            Registry::composer_for(Some(cargo)),
            Registry::toolchain(),
            Registry::repair(),
        ] {
            for tool in registry.tools() {
                let spec = tool.spec();
                for key in property_names(&spec.parameters) {
                    assert!(
                        !spec.description.contains(&format!("`{key}`")),
                        "{} description explains parameter `{key}`",
                        spec.name
                    );
                }
            }
        }
    }

    fn property_names(schema: &Value) -> Vec<String> {
        let mut names = Vec::new();
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            for (key, value) in properties {
                names.push(key.clone());
                names.extend(property_names(value));
            }
        }
        if let Some(items) = schema.get("items") {
            names.extend(property_names(items));
        }
        names
    }

    #[test]
    fn openai_strict_lists_every_property() {
        let cargo = crate::toolchain::Toolchain::parse("cargo").unwrap();
        for registry in [
            Registry::interpreter(),
            Registry::composer(),
            Registry::composer_for(Some(cargo)),
            Registry::toolchain(),
            Registry::repair(),
        ] {
            for tool in registry.tools() {
                let spec = tool.spec();
                assert_required_covers_properties(&spec.parameters, spec.name);
            }
        }
    }

    fn assert_required_covers_properties(schema: &Value, path: &str) {
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            let required = schema
                .get("required")
                .and_then(Value::as_array)
                .unwrap_or_else(|| panic!("{path}: missing required"));
            let required: Vec<&str> = required.iter().filter_map(Value::as_str).collect();
            for key in properties.keys() {
                assert!(
                    required.contains(&key.as_str()),
                    "{path}: `{key}` is not in required"
                );
                assert_required_covers_properties(&properties[key], &format!("{path}.{key}"));
            }
        }
        if let Some(items) = schema.get("items") {
            assert_required_covers_properties(items, &format!("{path}.items"));
        }
    }

    #[test]
    fn repair_names() {
        assert_eq!(
            Registry::repair().names(),
            vec![
                "list_source_files",
                "read_source_file",
                "write_output_file",
                "dream_error"
            ]
        );
        let write = Registry::repair()
            .tools()
            .iter()
            .find(|tool| tool.spec().name == "write_output_file")
            .unwrap()
            .spec();
        assert!(write.parameters["properties"].get("unit").is_none());
    }
}
