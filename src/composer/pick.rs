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
    entry_rel: &str,
    input: &mut Vec<serde_json::Value>,
) -> Result<Toolchain, DreamError> {
    let registry = Registry::toolchain();
    let instructions = prompt::toolchain(&registry);
    let mut pick_input = input.clone();
    pick_input.push(json!({
        "role": "user",
        "content": "Declare the toolchain before writing files."
    }));
    let turn = openai
        .respond(&instructions, &pick_input, &registry.schemas())
        .await?;
    if turn.function_calls.is_empty() {
        return Err(DreamError::composer(
            "toolchain was not declared; call set_toolchain",
        ));
    }

    let mut toolchain = None;
    for call in turn.function_calls {
        let mut ctx = ToolCtx::pick(project, deps, &mut toolchain);
        let _tool_output = dispatch(&registry, &mut ctx, &call)?;
    }
    let Some(known) = toolchain else {
        return Err(DreamError::composer(
            "toolchain was not declared; call set_toolchain",
        ));
    };
    input.push(json!({
        "role": "user",
        "content": known.declared_user_blob(entry_rel)?
    }));
    Ok(known)
}
