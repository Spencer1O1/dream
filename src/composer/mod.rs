pub(crate) mod output;
mod pick;
mod progress;
mod prompt;
mod repair;
mod session;

use std::path::Path;

use serde_json::json;

use crate::config::Config;
use crate::error::DreamError;
use crate::flags::ActiveFlags;
use crate::llm::OpenAi;
use crate::source::DepGraph;
use crate::source::Project;
use crate::tools::Registry;

use session::Session;

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

    let builder = session.ask_builder().await?;
    session.write_until_settled(staging.path()).await?;
    output::require_files(staging.path())?;
    output::replace_output(&output, staging.path())?;
    let _ = staging.keep();
    if opts.build || opts.run_program {
        session
            .build_and_repair(builder, &output, opts.run_program)
            .await?;
    }
    Ok(())
}
