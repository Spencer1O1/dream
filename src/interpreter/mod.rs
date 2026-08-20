mod prompt;

use std::path::Path;

use serde_json::{json, Value};

use crate::config::Config;
use crate::error::DreamError;
use crate::flags::ActiveFlags;
use crate::llm::{FunctionCall, OpenAi};
use crate::source::{DepGraph, Project};
use crate::tools::{Registry, ToolCtx};

pub async fn run(config: &Config, entry: &Path, strict: bool) -> Result<(), DreamError> {
    let (project, unit) = Project::from_entry(entry)?;
    let mut deps = DepGraph::new(&unit.rel);
    let openai = OpenAi::new(config.api_key.clone(), config.model.clone())?;
    let registry = Registry::interpreter();
    let flags = ActiveFlags::new(strict);
    let instructions = prompt::compose(&registry, &flags);
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

    Err(DreamError::interpreter(format!(
        "turn limit reached before the program settled ({})",
        config.turn_cap
    )))
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
        serde_json::from_str(&call.arguments).map_err(|_| {
            DreamError::runtime(format!("invalid arguments for tool `{}`", call.name))
        })?
    };
    let mut ctx = ToolCtx {
        project,
        deps,
        staging: None,
        builder: None,
    };
    registry.call(&call.name, &mut ctx, &args)
}
