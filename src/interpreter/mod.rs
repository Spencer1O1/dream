use std::path::Path;

use serde_json::{json, Value};

use crate::config::Config;
use crate::error::DreamError;
use crate::llm::{FunctionCall, OpenAi};
use crate::source::{DepGraph, Project};
use crate::tools::{Registry, ToolCtx};

const PREAMBLE: &str = "\
Your goal is to execute this Dream program as if it were actually running. Use the tools to do that.

A Dream program is foocode: informal notation in .foo files. \
One .foo file is one semantic unit. There is no grammar and no import keyword.

The entry unit is already in the conversation. \
Request other source units instead of inventing them.

Chat text is discarded. Do not chat.";

const STRICT_PROMPT: &str =
    "You are running in strict mode. Do not guess important ambiguity, return an error instead.";

const CLOSING: &str = "These tools are the entire interface. Anything else is invalid.";

pub async fn run(config: &Config, entry: &Path, strict: bool) -> Result<(), DreamError> {
    let (project, unit) = Project::from_entry(entry)?;
    let mut deps = DepGraph::new(&unit.rel);
    let openai = OpenAi::new(config.api_key.clone(), config.model.clone())?;
    let registry = Registry::interpreter();
    let instructions = compose_instructions(&registry, strict);
    let schemas = registry.schemas();

    let mut input = vec![json!({
        "role": "user",
        "content": format!(
            "Execute this Dream program.\n\nEntry: {}\n\n{}",
            unit.rel, unit.source
        )
    })];

    for _ in 0..config.turn_cap {
        let turn = openai.respond(&instructions, &input, &schemas).await?;
        if turn.function_calls.is_empty() {
            return Ok(());
        }

        input.extend(turn.output);

        for call in turn.function_calls {
            let output = dispatch(&registry, &project, &mut deps, &call)?;
            input.push(json!({
                "type": "function_call_output",
                "call_id": call.call_id,
                "output": output,
            }));
        }
    }

    Err(DreamError::new(format!(
        "turn limit reached before the program settled ({})",
        config.turn_cap
    )))
}

fn compose_instructions(registry: &Registry, strict: bool) -> String {
    let mut instructions = String::new();
    instructions.push_str(PREAMBLE);
    instructions.push_str("\n\n");
    instructions.push_str(&registry.prompt_catalog());
    instructions.push('\n');
    instructions.push_str(CLOSING);
    if strict {
        instructions.push_str("\n\n");
        instructions.push_str(STRICT_PROMPT);
    }
    instructions
}

fn dispatch(
    registry: &Registry,
    project: &Project,
    deps: &mut DepGraph,
    call: &FunctionCall,
) -> Result<String, DreamError> {
    let args: Value = if call.arguments.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&call.arguments)
            .map_err(|_| DreamError::new(format!("invalid arguments for tool `{}`", call.name)))?
    };
    let mut ctx = ToolCtx { project, deps };
    registry.call(&call.name, &mut ctx, &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instructions_include_registry_catalog() {
        let registry = Registry::interpreter();
        let instructions = compose_instructions(&registry, false);
        assert!(instructions.contains(PREAMBLE));
        assert!(instructions.contains(&registry.prompt_catalog()));
        assert!(instructions.contains(CLOSING));
        assert!(!instructions.contains(STRICT_PROMPT));
        assert!(compose_instructions(&registry, true).contains(STRICT_PROMPT));
    }
}
