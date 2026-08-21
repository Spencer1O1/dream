//! Compose write loop: respond, dispatch, settle.
//!
//! No preamble, no entry card, no repair message.

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::error::DreamError;
use crate::flags::ActiveFlags;
use crate::llm::OpenAi;
use crate::provenance;
use crate::source::{DepGraph, Project};
use crate::toolchain::Toolchain;
use crate::tools::{Compose, Registry, ToolCtx};

use super::dispatch::dispatch;
use super::state::ComposeState;

pub(crate) struct WriteLoop<'a> {
    pub artifacts: &'a mut HashMap<String, HashSet<String>>,
    pub repair: bool,
    pub toolchain: Option<Toolchain>,
    pub registry: &'a Registry,
    pub instructions: &'a str,
    pub schemas: &'a [Value],
}

pub(crate) struct Session<'a> {
    pub openai: &'a OpenAi,
    pub registry: &'a Registry,
    pub instructions: &'a str,
    pub schemas: &'a [Value],
    pub project: &'a Project,
    pub flags: &'a ActiveFlags,
    pub turn_cap: usize,
    pub repair_cap: usize,
    pub no_warn: bool,
    pub entry_rel: &'a str,
}

impl Session<'_> {
    pub async fn write_until_settled(
        &self,
        state: &mut ComposeState,
        deps: &mut DepGraph,
        input: &mut Vec<Value>,
        loop_state: WriteLoop<'_>,
    ) -> Result<(), DreamError> {
        let WriteLoop {
            artifacts,
            repair,
            toolchain,
            registry,
            instructions,
            schemas,
        } = loop_state;
        let mut missing_units_nudge = false;
        for _ in 0..self.turn_cap {
            let turn = self.openai.respond(instructions, input, schemas).await?;
            if turn.function_calls.is_empty() {
                if repair {
                    return Ok(());
                }
                match provenance::require_composed(artifacts, &state.store, &deps.reached_units()) {
                    Ok(()) => return Ok(()),
                    Err(err) if !missing_units_nudge => {
                        missing_units_nudge = true;
                        input.push(json!({
                            "role": "user",
                            "content": err.detail(),
                        }));
                        continue;
                    }
                    Err(err) => return Err(err),
                }
            }

            input.extend(turn.output);

            for call in turn.function_calls {
                let mut ctx = ToolCtx::compose(
                    self.project,
                    deps,
                    Compose {
                        dest: &state.dest,
                        store: &state.store,
                        artifacts,
                        toolchain,
                    },
                );
                let output = dispatch(registry, &mut ctx, &call)?;
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": call.call_id,
                    "output": output,
                }));
            }
        }

        Err(DreamError::composer(format!(
            "turn limit reached before composition settled ({})",
            self.turn_cap
        )))
    }
}
