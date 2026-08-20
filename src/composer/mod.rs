mod dispatch;
mod pick;
mod progress;
mod prompt;
mod repair;
mod session;
mod state;

use std::path::Path;

use serde_json::json;

use crate::config::Config;
use crate::error::DreamError;
use crate::flags::ActiveFlags;
use crate::llm::OpenAi;
use crate::output;
use crate::provenance;
use crate::source::DepGraph;
use crate::source::Project;
use crate::tools::Registry;

use session::Session;
use state::ComposeState;

pub struct RunOpts<'a> {
    pub entry: &'a Path,
    pub target: &'a str,
    pub output: &'a Path,
    pub strict: bool,
    pub no_warn: bool,
    pub build: bool,
    pub run_program: bool,
    pub fresh: bool,
}

pub async fn run(config: &Config, opts: RunOpts<'_>) -> Result<(), DreamError> {
    if opts.target.trim().is_empty() {
        return Err(DreamError::usage("compose requires -t <target>"));
    }

    let (project, unit) = Project::from_entry(opts.entry)?;
    let output = output::resolve_output_dir(project.root(), opts.output)?;
    let mut state = ComposeState::open(&output, opts.target, opts.fresh)?;
    if !state.fresh {
        provenance::check(&state.store, &state.dest, &project)?;
    }
    let mut deps = DepGraph::new(&unit.rel);
    let openai = OpenAi::new(config.api_key.clone(), config.model.clone())?;
    let flags = ActiveFlags::new(opts.strict);

    let mut input = vec![json!({
        "role": "user",
        "content": format!(
            "Compose this Dream program to {}.\n\nEntry: {}\n\n{}",
            opts.target, unit.rel, unit.source
        )
    })];

    let builder = pick::ask_builder(&openai, &project, &mut deps, &mut input).await?;

    let registry = Registry::composer_for(builder);
    let instructions = prompt::compose(&registry, &flags);
    let schemas = registry.schemas();
    let session = Session {
        openai: &openai,
        registry: &registry,
        instructions: &instructions,
        schemas: &schemas,
        project: &project,
        flags: &flags,
        turn_cap: config.turn_cap,
        repair_cap: config.repair_cap,
        no_warn: opts.no_warn,
    };

    if let Some(spec) = builder.and_then(crate::builder::Builder::spec) {
        crate::project::init(
            &state.dest,
            spec,
            &crate::project::from_entry(&unit.rel)?,
            &mut state.store,
        )?;
    }
    state
        .compose(&session, &mut deps, &mut input, builder)
        .await?;
    provenance::require_composed(&state.store)?;
    if opts.build || opts.run_program {
        session
            .build_and_repair(builder, &mut state, &mut input, &mut deps, opts.run_program)
            .await?;
    }
    Ok(())
}

pub fn lock(entry: &Path, target: &str, output: &Path) -> Result<(), DreamError> {
    if target.trim().is_empty() {
        return Err(DreamError::usage("lock requires -t <target>"));
    }
    let (project, unit) = Project::from_entry(entry)?;
    let output = output::resolve_output_dir(project.root(), output)?;
    provenance::lock(&output, target, &project, &unit.rel)
}

pub fn unlock(entry: &Path, target: &str, output: &Path) -> Result<(), DreamError> {
    if target.trim().is_empty() {
        return Err(DreamError::usage("unlock requires -t <target>"));
    }
    let (project, unit) = Project::from_entry(entry)?;
    let output = output::resolve_output_dir(project.root(), output)?;
    provenance::unlock(&output, target, &unit.rel)
}
