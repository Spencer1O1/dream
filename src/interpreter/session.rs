use serde_json::{json, Value};

use crate::error::DreamError;
use crate::llm::{FunctionCall, OpenAi};
use crate::source::{DepGraph, Project};
use crate::tools::{Registry, ToolCtx};

pub(crate) struct Session<'a> {
    pub openai: &'a OpenAi,
    pub registry: &'a Registry,
    pub instructions: &'a str,
    pub schemas: &'a [Value],
    pub project: &'a Project,
    pub deps: &'a mut DepGraph,
    pub input: &'a mut Vec<Value>,
    pub turn_cap: usize,
}

impl Session<'_> {
    pub async fn until_settled(&mut self) -> Result<(), DreamError> {
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
                let output = dispatch(self.registry, self.project, self.deps, &call)?;
                self.input.push(json!({
                    "type": "function_call_output",
                    "call_id": call.call_id,
                    "output": output,
                }));
            }
        }

        Err(DreamError::interpreter(format!(
            "turn limit reached before the program settled ({})",
            self.turn_cap
        )))
    }
}

fn dispatch(
    registry: &Registry,
    project: &Project,
    deps: &mut DepGraph,
    call: &FunctionCall,
) -> Result<String, DreamError> {
    let mut ctx = ToolCtx {
        project,
        deps,
        staging: None,
        builder: None,
    };
    registry.dispatch(&mut ctx, call)
}
