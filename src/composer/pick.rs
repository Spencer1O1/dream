use serde_json::json;

use crate::error::DreamError;
use crate::llm::OpenAi;
use crate::source::{DepGraph, Project};
use crate::toolchain::Toolchain;
use crate::tools::{Registry, ToolCtx};

use super::dispatch::dispatch;
use super::prompt;

pub(crate) async fn ask_toolchain(
    openai: &OpenAi,
    project: &Project,
    deps: &mut DepGraph,
    input: &mut Vec<serde_json::Value>,
) -> Result<Option<Toolchain>, DreamError> {
    let registry = Registry::toolchain();
    let instructions = prompt::toolchain(&registry);
    input.push(json!({
        "role": "user",
        "content": "Declare the toolchain before writing files."
    }));
    let turn = openai
        .respond(&instructions, input, &registry.schemas())
        .await?;
    input.extend(turn.output);
    if turn.function_calls.is_empty() {
        return Ok(None);
    }

    let mut toolchain = None;
    for call in turn.function_calls {
        let mut ctx = ToolCtx::pick(project, deps, &mut toolchain);
        let tool_output = dispatch(&registry, &mut ctx, &call)?;
        input.push(json!({
            "type": "function_call_output",
            "call_id": call.call_id,
            "output": tool_output,
        }));
    }
    Ok(toolchain)
}
