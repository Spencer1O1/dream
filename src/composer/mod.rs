pub(crate) mod output;
mod prompt;

use std::path::Path;

use serde_json::{json, Value};

use crate::builder::{Builder, Outcome};
use crate::config::Config;
use crate::error::DreamError;
use crate::flags::ActiveFlags;
use crate::llm::{FunctionCall, OpenAi};
use crate::source::{DepGraph, Project};
use crate::tools::{Registry, ToolCtx};

pub struct RunOpts<'a> {
    pub entry: &'a Path,
    pub target: &'a str,
    pub output: &'a Path,
    pub strict: bool,
    pub no_warn: bool,
    pub build: bool,
    pub run_program: bool,
}

pub async fn run(config: &Config, opts: RunOpts<'_>) -> Result<(), DreamError> {
    if opts.target.trim().is_empty() {
        return Err(DreamError::usage("compose requires -t <target>"));
    }

    let (project, unit) = Project::from_entry(opts.entry)?;
    let output = output::resolve_output_dir(project.root(), opts.output)?;
    let staging = tempfile::tempdir()?;
    let mut deps = DepGraph::new(&unit.rel);
    let openai = OpenAi::new(config.api_key.clone(), config.model.clone())?;
    let registry = Registry::composer();
    let flags = ActiveFlags::new(opts.strict, opts.no_warn);
    let instructions = prompt::compose(&registry, &flags);
    let schemas = registry.schemas();

    let mut input = vec![json!({
        "role": "user",
        "content": format!(
            "Compose this Dream program to {}.\n\nEntry: {}\n\n{}",
            opts.target, unit.rel, unit.source
        )
    })];

    let mut session = Session {
        openai: &openai,
        registry: &registry,
        instructions: &instructions,
        schemas: &schemas,
        project: &project,
        deps: &mut deps,
        input: &mut input,
        flags: &flags,
        turn_cap: config.turn_cap,
        repair_cap: config.repair_cap,
        no_warn: opts.no_warn,
    };

    session.write_until_settled(staging.path()).await?;
    require_files(staging.path())?;
    let builder = session.ask_builder().await?;
    output::replace_output(&output, staging.path())?;
    let _ = staging.keep();
    if opts.build || opts.run_program {
        session
            .build_and_repair(builder, &output, opts.run_program)
            .await?;
    }
    Ok(())
}

struct Session<'a> {
    openai: &'a OpenAi,
    registry: &'a Registry,
    instructions: &'a str,
    schemas: &'a [Value],
    project: &'a Project,
    deps: &'a mut DepGraph,
    input: &'a mut Vec<Value>,
    flags: &'a ActiveFlags,
    turn_cap: usize,
    repair_cap: usize,
    no_warn: bool,
}

impl Session<'_> {
    async fn write_until_settled(&mut self, staging: &Path) -> Result<(), DreamError> {
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

    async fn ask_builder(&mut self) -> Result<Option<Builder>, DreamError> {
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

    async fn build_and_repair(
        &mut self,
        builder: Option<Builder>,
        output: &Path,
        run_program: bool,
    ) -> Result<(), DreamError> {
        for attempt in 0..=self.repair_cap {
            match crate::builder::after_compose(builder, output, run_program, self.no_warn)? {
                Outcome::Ok => return Ok(()),
                Outcome::Failed { step, diagnostics }
                    if should_repair(attempt, step, self.repair_cap) =>
                {
                    self.input.push(json!({
                        "role": "user",
                        "content": repair_message(&diagnostics),
                    }));
                    self.write_until_settled(output).await?;
                    require_files(output)?;
                }
                outcome => return outcome.into_error(),
            }
        }
        Err(DreamError::runtime("build failed"))
    }
}

fn should_repair(attempt: usize, step: &str, cap: usize) -> bool {
    step == "build" && attempt < cap
}

fn repair_message(diagnostics: &str) -> String {
    let diagnostics = diagnostics.trim();
    if diagnostics.is_empty() {
        "Build failed. Repair the project.".to_string()
    } else {
        format!("Build failed. Repair the project.\n\n{diagnostics}")
    }
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

    #[test]
    fn only_build_failures_repair_and_only_under_the_cap() {
        assert!(should_repair(0, "build", 3));
        assert!(should_repair(2, "build", 3));
        assert!(!should_repair(3, "build", 3));
        assert!(!should_repair(0, "build", 0));
        assert!(!should_repair(0, "run", 3));
    }

    #[test]
    fn repair_message_includes_diagnostics() {
        assert_eq!(repair_message("   "), "Build failed. Repair the project.");
        assert!(repair_message("error: nope").contains("error: nope"));
    }
}
