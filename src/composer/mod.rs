pub(crate) mod output;
mod prompt;

use std::path::Path;

use serde_json::{json, Value};

use crate::builder::Builder;
use crate::config::Config;
use crate::error::DreamError;
use crate::flags::ActiveFlags;
use crate::llm::{FunctionCall, OpenAi};
use crate::source::{DepGraph, Project};
use crate::tools::{Registry, ToolCtx};

pub async fn run(
    config: &Config,
    entry: &Path,
    target: &str,
    output: &Path,
    strict: bool,
    build: bool,
    run_program: bool,
) -> Result<(), DreamError> {
    if target.trim().is_empty() {
        return Err(DreamError::usage("compose requires -t <target>"));
    }

    let (project, unit) = Project::from_entry(entry)?;
    let output = output::resolve_output_dir(project.root(), output)?;
    let staging = tempfile::tempdir()?;
    let mut deps = DepGraph::new(&unit.rel);
    let openai = OpenAi::new(config.api_key.clone(), config.model.clone())?;
    let registry = Registry::composer();
    let flags = ActiveFlags::new(strict);
    let instructions = prompt::compose(&registry, &flags);
    let schemas = registry.schemas();

    let mut input = vec![json!({
        "role": "user",
        "content": format!(
            "Compose this Dream program to {target}.\n\nEntry: {}\n\n{}",
            unit.rel, unit.source
        )
    })];

    for _ in 0..config.turn_cap {
        let turn = openai.respond(&instructions, &input, &schemas).await?;
        if turn.function_calls.is_empty() {
            let builder = finish(
                &openai, &project, &mut deps, staging, &mut input, &flags, &output,
            )
            .await?;
            if build || run_program {
                crate::builder::after_compose(builder, &output, run_program)?;
            }
            return Ok(());
        }

        input.extend(turn.output);

        for call in turn.function_calls {
            let tool_output = dispatch(
                &registry,
                &project,
                &mut deps,
                Some(staging.path()),
                None,
                &call,
            )?;
            input.push(json!({
                "type": "function_call_output",
                "call_id": call.call_id,
                "output": tool_output,
            }));
        }
    }

    Err(DreamError::runtime(format!(
        "turn limit reached before composition settled ({})",
        config.turn_cap
    )))
}

async fn finish(
    openai: &OpenAi,
    project: &Project,
    deps: &mut DepGraph,
    staging: tempfile::TempDir,
    input: &mut Vec<Value>,
    flags: &ActiveFlags,
    output: &Path,
) -> Result<Option<Builder>, DreamError> {
    require_files(staging.path())?;
    let builder = ask_builder(openai, project, deps, input, flags).await?;
    output::replace_output(output, staging.path())?;
    let _ = staging.keep();
    Ok(builder)
}

async fn ask_builder(
    openai: &OpenAi,
    project: &Project,
    deps: &mut DepGraph,
    input: &mut Vec<Value>,
    flags: &ActiveFlags,
) -> Result<Option<Builder>, DreamError> {
    let registry = Registry::builder();
    let instructions = prompt::builder(&registry, flags);
    input.push(json!({
        "role": "user",
        "content": "Declare the toolchain for this project."
    }));
    let turn = openai
        .respond(&instructions, input, &registry.schemas())
        .await?;
    if turn.function_calls.is_empty() {
        return Ok(None);
    }

    let mut builder = None;
    for call in turn.function_calls {
        dispatch(&registry, project, deps, None, Some(&mut builder), &call)?;
    }
    Ok(builder)
}

fn require_files(staging: &Path) -> Result<(), DreamError> {
    if !output::tree_has_files(staging)? {
        return Err(DreamError::runtime("composition produced no files"));
    }
    Ok(())
}

fn dispatch(
    registry: &Registry,
    project: &Project,
    deps: &mut DepGraph,
    staging: Option<&Path>,
    builder: Option<&mut Option<Builder>>,
    call: &FunctionCall,
) -> Result<String, DreamError> {
    let mut ctx = ToolCtx {
        project,
        deps,
        staging,
        builder,
    };
    registry.dispatch(&mut ctx, call)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_files_is_an_error() {
        let staging = tempfile::tempdir().unwrap();
        let err = require_files(staging.path()).unwrap_err();
        assert!(err.to_string().contains("produced no files"));
    }
}
