use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::builder::Builder;
use crate::composer::provenance::Dependency;
use crate::error::DreamError;
use crate::flags::ActiveFlags;
use crate::llm::OpenAi;
use crate::source::{DepGraph, Project};
use crate::tools::{Registry, WriteSlot};

use super::dispatch::{dispatch, ToolIo};
use super::state::ComposeState;

pub(crate) struct WriteLoop<'a> {
    pub artifacts: &'a mut HashMap<String, HashSet<String>>,
    pub dependencies: &'a mut HashMap<String, Vec<Dependency>>,
    pub repair: bool,
    pub toolchain: Option<Builder>,
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
        } = loop_state;
        for _ in 0..self.turn_cap {
            let turn = self
                .openai
                .respond(self.instructions, input, self.schemas)
                .await?;
            if turn.function_calls.is_empty() {
                return Ok(());
            }

            input.extend(turn.output);

            for call in turn.function_calls {
                let output = dispatch(
                    self.registry,
                    self.project,
                    deps,
                    ToolIo {
                        dest: Some(&state.dest),
                        store: Some(&state.store),
                        write: if repair {
                            Some(WriteSlot::Repair)
                        } else {
                            Some(WriteSlot::Compose {
                                artifacts,
                                dependencies,
                                fresh: state.fresh,
                            })
                        },
                        builder: None,
                        toolchain,
                    },
                    &call,
                )?;
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": call.call_id,
                    "output": output,
                }));
            }
        }

        Err(DreamError::runtime(format!(
            "turn limit reached before composition settled ({})",
            self.turn_cap
        )))
    }
}
