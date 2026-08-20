pub(crate) mod output;
mod prompt;

use std::path::Path;

use serde_json::{json, Value};

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
            return settle(&output, staging);
        }

        input.extend(turn.output);

        for call in turn.function_calls {
            let tool_output = dispatch(&registry, &project, &mut deps, staging.path(), &call)?;
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

fn settle(output: &Path, staging: tempfile::TempDir) -> Result<(), DreamError> {
    if !output::tree_has_files(staging.path())? {
        return Err(DreamError::runtime("composition produced no files"));
    }
    output::replace_output(output, staging.path())?;
    let _ = staging.keep();
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
    fn settle_without_files_leaves_destination() {
        let parent = tempfile::tempdir().unwrap();
        let dest = parent.path().join("out");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("keep.txt"), "keep").unwrap();
        let staging = tempfile::tempdir().unwrap();
        let err = settle(&dest, staging).unwrap_err();
        assert!(err.to_string().contains("produced no files"));
        assert_eq!(fs::read_to_string(dest.join("keep.txt")).unwrap(), "keep");
    }
}
