use serde_json::json;

use crate::builder::Builder;
use crate::error::DreamError;
use crate::source::DepGraph;
use crate::tools::Registry;

use super::dispatch::{dispatch, ToolIo};
use super::prompt;
use super::session::Session;

impl Session<'_> {
    pub async fn ask_builder(
        &self,
        deps: &mut DepGraph,
        input: &mut Vec<serde_json::Value>,
    ) -> Result<Option<Builder>, DreamError> {
        let registry = Registry::builder();
        let instructions = prompt::builder(&registry, self.flags);
        input.push(json!({
            "role": "user",
            "content": "Declare the toolchain before writing files."
        }));
        let turn = self
            .openai
            .respond(&instructions, input, &registry.schemas())
            .await?;
        input.extend(turn.output);
        if turn.function_calls.is_empty() {
            return Ok(None);
        }

        let mut builder = None;
        for call in turn.function_calls {
            let tool_output = dispatch(
                &registry,
                self.project,
                deps,
                ToolIo {
                    dest: None,
                    store: None,
                    write: None,
                    builder: Some(&mut builder),
                    toolchain: None,
                },
                &call,
            )?;
            input.push(json!({
                "type": "function_call_output",
                "call_id": call.call_id,
                "output": tool_output,
            }));
        }
        Ok(builder)
    }
}
