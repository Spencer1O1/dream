use serde_json::json;

use crate::error::DreamError;
use crate::source::DepGraph;
use crate::toolchain::{Outcome, Toolchain};
use crate::tools::Registry;

use super::progress;
use super::prompt;
use super::session::Session;
use super::state::ComposeState;

impl Session<'_> {
    pub async fn build_and_repair(
        &self,
        toolchain: Option<Toolchain>,
        state: &mut ComposeState,
        deps: &mut DepGraph,
        run_program: bool,
    ) -> Result<(), DreamError> {
        let mut attempt = 0;
        loop {
            match crate::toolchain::after_compose(
                toolchain,
                &state.dest,
                self.entry_rel,
                run_program,
                self.no_warn,
            )? {
                Outcome::Ok => return Ok(()),
                Outcome::Failed { step, diagnostics }
                    if should_repair(attempt, step, self.repair_cap) =>
                {
                    progress::repair();
                    let mut repair_input = Vec::new();
                    if let Some(known) = toolchain {
                        repair_input.push(json!({
                            "role": "user",
                            "content": known.declared_user_blob(self.entry_rel)?,
                        }));
                    }
                    repair_input.push(json!({
                        "role": "user",
                        "content": repair_message(&diagnostics),
                    }));
                    let mut artifacts = std::collections::HashMap::new();
                    let registry = Registry::repair();
                    let instructions = prompt::repair(&registry, self.flags, toolchain);
                    let schemas = registry.schemas();
                    self.write_until_settled(
                        state,
                        deps,
                        &mut repair_input,
                        super::session::WriteLoop {
                            artifacts: &mut artifacts,
                            repair: true,
                            toolchain,
                            registry: &registry,
                            instructions: &instructions,
                            schemas: &schemas,
                        },
                    )
                    .await?;
                    attempt += 1;
                }
                outcome => return outcome.into_error(),
            }
        }
    }
}

fn should_repair(attempt: usize, step: &str, cap: usize) -> bool {
    (step == "configure" || step == "build") && attempt < cap
}

fn repair_message(diagnostics: &str) -> String {
    let diagnostics = diagnostics.trim();
    if diagnostics.is_empty() {
        "Build failed. Repair dest files. Write this toolchain's setup files if that is what the diagnostics need.".to_string()
    } else {
        format!(
            "Build failed. Repair dest files. Write this toolchain's setup files if that is what the diagnostics need.\n\n{diagnostics}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_build_failures_repair_and_only_under_the_cap() {
        assert!(should_repair(0, "build", 3));
        assert!(should_repair(0, "configure", 3));
        assert!(should_repair(2, "build", 3));
        assert!(!should_repair(3, "build", 3));
        assert!(!should_repair(0, "build", 0));
        assert!(!should_repair(0, "run", 3));
    }

    #[test]
    fn repair_message_includes_diagnostics() {
        assert!(repair_message("   ").contains("Repair dest files"));
        assert!(repair_message("error: nope").contains("error: nope"));
    }
}
