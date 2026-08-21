//! Repair job: when to retry, a new stack, merge writes.
//!
//! Calls `prompt::repair` for standing law. Owns the this-run failure card.
//! Does not continue the compose transcript. No standing law of its own.

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
                    let entry = self.project.read_foo_file(self.entry_rel)?;
                    let mut repair_input =
                        repair_stack(&entry.rel, &entry.source, toolchain, &diagnostics)?;
                    let mut artifacts = std::collections::HashMap::new();
                    let registry = Registry::composer_for(toolchain);
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
                    state.merge_writes(artifacts)?;
                    attempt += 1;
                }
                outcome => return outcome.into_error(),
            }
        }
    }
}

fn repair_stack(
    entry_rel: &str,
    source: &str,
    toolchain: Option<Toolchain>,
    diagnostics: &str,
) -> Result<Vec<serde_json::Value>, DreamError> {
    let mut input = prompt::this_run(entry_rel, source, toolchain)?;
    input.push(json!({
        "role": "user",
        "content": repair_message(diagnostics),
    }));
    Ok(input)
}

fn should_repair(attempt: usize, step: &str, cap: usize) -> bool {
    (step == "configure" || step == "build") && attempt < cap
}

fn repair_message(diagnostics: &str) -> String {
    let diagnostics = diagnostics.trim();
    if diagnostics.is_empty() {
        "Configure or build failed.".to_string()
    } else {
        format!("Configure or build failed.\n\n{diagnostics}")
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
        assert_eq!(repair_message("   "), "Configure or build failed.");
        assert!(repair_message("error: nope").contains("error: nope"));
    }

    #[test]
    fn repair_stack_has_the_entry_and_the_diagnostics() {
        let cargo = Toolchain::parse("cargo").unwrap();
        let stack = repair_stack(
            "limits.foo",
            "print far origin near",
            Some(cargo),
            "error: missing Cargo.toml",
        )
        .unwrap();
        let first = stack[0]["content"].as_str().unwrap();
        assert!(!first.contains("Compose this Dream program"));
        assert!(first.contains("Entry `.foo` file: limits.foo"));
        assert!(first.contains("print far origin near"));
        assert!(stack[1]["content"]
            .as_str()
            .unwrap()
            .contains("Toolchain cargo"));
        let last = stack[2]["content"].as_str().unwrap();
        assert!(last.contains("Configure or build failed"));
        assert!(last.contains("error: missing Cargo.toml"));
    }
}
