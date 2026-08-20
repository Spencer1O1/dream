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
    refuse_unimplemented_build(build, run_program)?;
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
            return finish(
                &openai, &project, &mut deps, staging, &mut input, &flags, &output,
            )
            .await;
        }

        input.extend(turn.output);

        for call in turn.function_calls {
            let tool_output =
                dispatch(&registry, &project, &mut deps, staging.path(), None, &call)?;
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
) -> Result<(), DreamError> {
    require_files(staging.path())?;
    let _builder = ask_builder(openai, project, deps, staging.path(), input, flags).await?;
    output::replace_output(output, staging.path())?;
    let _ = staging.keep();
    Ok(())
}

async fn ask_builder(
    openai: &OpenAi,
    project: &Project,
    deps: &mut DepGraph,
    staging: &Path,
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

    input.extend(turn.output);
    let mut builder = None;
    for call in turn.function_calls {
        let tool_output = dispatch(&registry, project, deps, staging, Some(&mut builder), &call)?;
        input.push(json!({
            "type": "function_call_output",
            "call_id": call.call_id,
            "output": tool_output,
        }));
    }
    Ok(builder)
}

fn require_files(staging: &Path) -> Result<(), DreamError> {
    if !output::tree_has_files(staging)? {
        return Err(DreamError::runtime("composition produced no files"));
    }
    Ok(())
}

fn refuse_unimplemented_build(build: bool, run_program: bool) -> Result<(), DreamError> {
    if build || run_program {
        return Err(DreamError::usage(
            "composition does not build yet; omit --build and --run",
        ));
    }
    Ok(())
}

fn dispatch(
    registry: &Registry,
    project: &Project,
    deps: &mut DepGraph,
    staging: &Path,
    builder: Option<&mut Option<Builder>>,
    call: &FunctionCall,
) -> Result<String, DreamError> {
    let args: Value = if call.arguments.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&call.arguments).map_err(|_| {
            DreamError::runtime(format!("invalid arguments for tool `{}`", call.name))
        })?
    };
    let mut ctx = ToolCtx {
        project,
        deps,
        staging: Some(staging),
        builder,
    };
    registry.call(&call.name, &mut ctx, &args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn build_and_run_are_not_implemented() {
        let err = refuse_unimplemented_build(true, false).unwrap_err();
        assert!(err.to_string().contains("omit --build"));
        assert!(refuse_unimplemented_build(false, false).is_ok());
    }

    #[test]
    fn no_files_leaves_destination() {
        let parent = tempfile::tempdir().unwrap();
        let dest = parent.path().join("out");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("keep.txt"), "keep").unwrap();
        let staging = tempfile::tempdir().unwrap();
        let err = require_files(staging.path()).unwrap_err();
        assert!(err.to_string().contains("produced no files"));
        assert_eq!(fs::read_to_string(dest.join("keep.txt")).unwrap(), "keep");
    }
}
