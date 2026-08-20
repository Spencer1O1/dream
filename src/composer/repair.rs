use serde_json::json;

use crate::builder::{Builder, Outcome};
use crate::error::DreamError;
use crate::provenance;
use crate::source::DepGraph;
use crate::tools::Registry;

use super::progress;
use super::prompt;
use super::session::Session;
use super::state::ComposeState;

impl Session<'_> {
    pub async fn build_and_repair(
        &self,
        builder: Option<Builder>,
        state: &mut ComposeState,
        input: &mut Vec<serde_json::Value>,
        deps: &mut DepGraph,
        run_program: bool,
    ) -> Result<(), DreamError> {
        for attempt in 0..=self.repair_cap {
            match crate::builder::after_compose(builder, &state.dest, run_program, self.no_warn)? {
                Outcome::Ok => return Ok(()),
                Outcome::Failed { step, diagnostics }
                    if should_repair(attempt, step, self.repair_cap) =>
                {
                    progress::repair();
                    input.push(json!({
                        "role": "user",
                        "content": repair_message(&diagnostics),
                    }));
                    let mut artifacts = std::collections::HashMap::new();
                    let mut dependencies = std::collections::HashMap::new();
                    let registry = Registry::repair();
                    let instructions = prompt::repair(&registry, self.flags);
                    let schemas = registry.schemas();
                    self.write_until_settled(
                        state,
                        deps,
                        input,
                        super::session::WriteLoop {
                            artifacts: &mut artifacts,
                            dependencies: &mut dependencies,
                            repair: true,
                            toolchain: builder,
                            registry: &registry,
                            instructions: &instructions,
                            schemas: &schemas,
                        },
                    )
                    .await?;
                    provenance::require_composed(&state.store)?;
                }
                outcome => return outcome.into_error(),
            }
        }
        Err(DreamError::composer("build failed"))
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

#[cfg(test)]
mod tests {
    use super::*;

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
