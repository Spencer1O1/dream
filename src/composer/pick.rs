//! Pick turn: declare a catalog row from the `-t` hint.
//!
//! The pick stack is the entry plus the requested-toolchain card.
//! The compose stack gets the chosen row as a fact, not the hint.
//! Instructions come from `prompt::toolchain`. No standing law of its own.

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
    input: &[serde_json::Value],
) -> Result<Toolchain, DreamError> {
    let registry = Registry::toolchain();
    let instructions = prompt::toolchain(&registry);
    crate::trace::job("pick", &instructions, input);
    let turn = openai
        .respond(&instructions, input, &registry.schemas())
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
    Ok(known)
}
