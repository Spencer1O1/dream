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

    pub fn composer_for(builder: Option<crate::builder::Builder>) -> Self {
        if builder.and_then(|known| known.spec()).is_some() {
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

    pub fn builder() -> Self {
        Self::gather(&[super::builder::tools])
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
            Registry::composer_for(Some(crate::builder::Builder::parse("cargo").unwrap())).names(),
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
    fn builder_names() {
        assert_eq!(Registry::builder().names(), vec!["set_builder"]);
    }

    #[test]
    fn openai_strict_lists_every_property() {
        let cargo = crate::builder::Builder::parse("cargo").unwrap();
        for registry in [
            Registry::interpreter(),
            Registry::composer(),
            Registry::composer_for(Some(cargo)),
            Registry::builder(),
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
