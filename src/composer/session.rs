use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::builder::Builder;
use crate::error::DreamError;
use crate::flags::ActiveFlags;
use crate::llm::OpenAi;
use crate::provenance::Dependency;
use crate::source::{DepGraph, Project};
use crate::tools::{Compose, Registry, ToolCtx};

use super::dispatch::dispatch;
use super::state::ComposeState;

pub(crate) struct WriteLoop<'a> {
    pub artifacts: &'a mut HashMap<String, HashSet<String>>,
    pub dependencies: &'a mut HashMap<String, Vec<Dependency>>,
    pub repair: bool,
    pub toolchain: Option<Builder>,
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
            dependencies,
            repair,
            toolchain,
            registry,
            instructions,
            schemas,
        } = loop_state;
        for _ in 0..self.turn_cap {
            let turn = self.openai.respond(instructions, input, schemas).await?;
            if turn.function_calls.is_empty() {
                return Ok(());
            }

            input.extend(turn.output);

            for call in turn.function_calls {
                let mut ctx = if repair {
                    ToolCtx::repair(self.project, deps, &state.dest, &state.store)
                } else {
                    ToolCtx::compose(
                        self.project,
                        deps,
                        Compose {
                            dest: &state.dest,
                            store: &state.store,
                            artifacts,
                            dependencies,
                            toolchain,
                        },
                    )
                };
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
