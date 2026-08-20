use serde_json::json;

use crate::builder::Builder;
use crate::error::DreamError;
use crate::tools::Registry;

use super::prompt;
use super::session::{dispatch, Session};

impl Session<'_> {
    pub async fn ask_builder(&mut self) -> Result<Option<Builder>, DreamError> {
        let registry = Registry::builder();
        let instructions = prompt::builder(&registry, self.flags);
        self.input.push(json!({
            "role": "user",
            "content": "Declare the toolchain for this project."
        }));
        let turn = self
            .openai
            .respond(&instructions, self.input, &registry.schemas())
            .await?;
        self.input.extend(turn.output);
        if turn.function_calls.is_empty() {
            return Ok(None);
        }

        let mut builder = None;
        for call in turn.function_calls {
            let tool_output = dispatch(
                &registry,
                self.project,
                self.deps,
                None,
                Some(&mut builder),
                &call,
            )?;
            self.input.push(json!({
                "type": "function_call_output",
                "call_id": call.call_id,
                "output": tool_output,
            }));
        }
        Ok(builder)
    }
}
