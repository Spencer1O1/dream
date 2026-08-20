mod prompt;
mod session;

use std::path::Path;

use serde_json::json;

use crate::config::Config;
use crate::error::DreamError;
use crate::flags::ActiveFlags;
use crate::llm::OpenAi;
use crate::source::{DepGraph, Project};
use crate::tools::Registry;

use session::Session;

pub async fn run(config: &Config, entry: &Path, strict: bool) -> Result<(), DreamError> {
    let (project, unit) = Project::from_entry(entry)?;
    let mut deps = DepGraph::new(&unit.rel);
    let openai = OpenAi::new(config.api_key.clone(), config.model.clone())?;
    let registry = Registry::interpreter();
    let flags = ActiveFlags::new(strict, false);
    let instructions = prompt::compose(&registry, &flags);
    let schemas = registry.schemas();

    let mut input = vec![json!({
        "role": "user",
        "content": format!(
            "Execute this Dream program.\n\nEntry: {}\n\n{}",
            unit.rel, unit.source
        )
    })];

    Session {
        openai: &openai,
        registry: &registry,
        instructions: &instructions,
        schemas: &schemas,
        project: &project,
        deps: &mut deps,
        input: &mut input,
        turn_cap: config.turn_cap,
    }
    .until_settled()
    .await
}
