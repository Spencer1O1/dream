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
use crate::toolchain::Toolchain;
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

    let (project, unit) = Project::from_path(opts.entry)?;
    let output = output::resolve_output_dir(project.root(), opts.output)?;
    let mut state = ComposeState::open(&output, opts.target, opts.fresh)?;
    if !state.fresh {
        provenance::require_source_root(&state.store, project.root())?;
        provenance::check(&state.store, &state.dest, &project)?;
    }
    state.store.set_source_root(project.root())?;
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

    let toolchain = match Toolchain::parse(opts.target) {
        Ok(known) => {
            input.push(json!({
                "role": "user",
                "content": format!(
                    "Toolchain {}:\n{}",
                    known.as_str(),
                    known.declared(&unit.rel)?
                )
            }));
            Some(known)
        }
        Err(_) => pick::ask_toolchain(&openai, &project, &mut deps, &mut input).await?,
    };

    let registry = Registry::composer_for(toolchain);
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
        entry_rel: &unit.rel,
    };

    if let Some(spec) = toolchain.and_then(crate::toolchain::Toolchain::spec) {
        crate::dest::init(&state.dest, spec, &mut state.store)?;
    }
    state
        .compose(&session, &mut deps, &mut input, toolchain)
        .await?;
    if opts.build || opts.run_program {
        session
            .build_and_repair(
                toolchain,
                &mut state,
                &mut input,
                &mut deps,
                opts.run_program,
            )
            .await?;
    }
    Ok(())
}

pub fn lock(entry: &Path, target: &str, output: &Path) -> Result<(), DreamError> {
    if target.trim().is_empty() {
        return Err(DreamError::usage("lock requires -t <target>"));
    }
    let (project, unit) = Project::from_path(entry)?;
    let output = output::resolve_output_dir(project.root(), output)?;
    provenance::lock(&output, target, &project.root().join(&unit.rel))
}

pub fn unlock(entry: &Path, target: &str, output: &Path) -> Result<(), DreamError> {
    if target.trim().is_empty() {
        return Err(DreamError::usage("unlock requires -t <target>"));
    }
    let (project, unit) = Project::from_path(entry)?;
    let output = output::resolve_output_dir(project.root(), output)?;
    provenance::unlock(&output, target, &project.root().join(&unit.rel))
}

pub fn inspect(path: &Path, target: &str, output: &Path) -> Result<(), DreamError> {
    if target.trim().is_empty() {
        return Err(DreamError::usage("inspect requires -t <target>"));
    }
    let (project, unit) = if path.is_dir() {
        (Project::from_root(path)?, None)
    } else {
        let (project, unit) = Project::from_entry(path)?;
        (project, Some(unit.rel))
    };
    let output = output::resolve_output_dir(project.root(), output)?;
    print!(
        "{}",
        provenance::inspect(&output, target, &project, unit.as_deref())?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::Store;
    use std::collections::HashSet;
    use std::fs;

    #[test]
    fn lock_nested_unit_uses_the_store_key() {
        let src = tempfile::tempdir().unwrap();
        fs::create_dir_all(src.path().join("users")).unwrap();
        fs::write(src.path().join("main.foo"), "entry").unwrap();
        fs::write(src.path().join("users/active.foo"), "active").unwrap();

        let dest = tempfile::tempdir().unwrap();
        fs::create_dir_all(dest.path().join("src")).unwrap();
        fs::write(dest.path().join("src/active.rs"), "fn active() {}").unwrap();
        let mut store = Store::new("rust");
        store.set_source_root(src.path()).unwrap();
        store.set_artifacts("users/active.foo", HashSet::from(["src/active.rs".into()]));
        store.save(dest.path()).unwrap();

        lock(&src.path().join("users/active.foo"), "rust", dest.path()).unwrap();

        let store = Store::load(dest.path()).unwrap().unwrap();
        assert!(store.is_locked("users/active.foo"));
        assert!(!store.units.contains_key("active.foo"));

        unlock(&src.path().join("users/active.foo"), "rust", dest.path()).unwrap();
        let store = Store::load(dest.path()).unwrap().unwrap();
        assert!(!store.is_locked("users/active.foo"));
    }
}
