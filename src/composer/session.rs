use std::path::Path;

use serde_json::{json, Value};

use crate::builder::Builder;
use crate::error::DreamError;
use crate::flags::ActiveFlags;
use crate::llm::{FunctionCall, OpenAi};
use crate::source::{DepGraph, Project};
use crate::tools::{Registry, ToolCtx};

use super::progress;

pub(crate) struct Session<'a> {
    pub openai: &'a OpenAi,
    pub registry: &'a Registry,
    pub instructions: &'a str,
    pub schemas: &'a [Value],
    pub project: &'a Project,
    pub deps: &'a mut DepGraph,
    pub input: &'a mut Vec<Value>,
    pub flags: &'a ActiveFlags,
    pub turn_cap: usize,
    pub repair_cap: usize,
    pub no_warn: bool,
}

impl Session<'_> {
    pub async fn write_until_settled(&mut self, staging: &Path) -> Result<(), DreamError> {
        for _ in 0..self.turn_cap {
            let turn = self
                .openai
                .respond(self.instructions, self.input, self.schemas)
                .await?;
            if turn.function_calls.is_empty() {
                return Ok(());
            }

            self.input.extend(turn.output);

            for call in turn.function_calls {
                let tool_output = dispatch(
                    self.registry,
                    self.project,
                    self.deps,
                    Some(staging),
                    None,
                    &call,
                )?;
                self.input.push(json!({
                    "type": "function_call_output",
                    "call_id": call.call_id,
                    "output": tool_output,
                }));
            }
        }

        Err(DreamError::runtime(format!(
            "turn limit reached before composition settled ({})",
            self.turn_cap
        )))
    }
}

pub(crate) fn dispatch(
    registry: &Registry,
    project: &Project,
    deps: &mut DepGraph,
    staging: Option<&Path>,
    builder: Option<&mut Option<Builder>>,
    call: &FunctionCall,
) -> Result<String, DreamError> {
    let args = call.parsed_args()?;
    progress::tool(&call.name, &args);
    let mut ctx = ToolCtx {
        project,
        deps,
        staging,
        builder,
    };
    registry.dispatch(&mut ctx, call)
}
