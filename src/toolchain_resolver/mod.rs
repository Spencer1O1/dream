//! Map `-t` to a catalog row or `unsupported`.
//!
//! A catalog name binds immediately. A matching store reuses that bind.
//! Otherwise one model turn calls `set_toolchain`.
//! Compose gets the chosen toolchain as a fact, or the requested target
//! when there is no catalog row.

mod prompt;

use crate::error::DreamError;
use crate::llm::OpenAi;
use crate::provenance;
use crate::source::{DepGraph, Project};
use crate::toolchain::Toolchain;
use crate::tools::{Registry, ToolCtx};

/// Catalog name or a reusable store bind. `None` means ask the model.
pub fn from_store_or_catalog(target: &str, stored: &str, fresh: bool) -> Option<Toolchain> {
    if let Ok(known) = Toolchain::parse(target) {
        return Some(known);
    }
    provenance::existing_bind(stored, target, fresh)
}

pub async fn resolve(
    target: &str,
    stored: &str,
    fresh: bool,
    openai: &OpenAi,
    project: &Project,
    deps: &mut DepGraph,
) -> Result<Toolchain, DreamError> {
    if let Some(known) = from_store_or_catalog(target, stored, fresh) {
        return Ok(known);
    }
    ask(openai, project, deps, target).await
}

async fn ask(
    openai: &OpenAi,
    project: &Project,
    deps: &mut DepGraph,
    target: &str,
) -> Result<Toolchain, DreamError> {
    let registry = Registry::toolchain();
    let instructions = prompt::instructions(&registry);
    let input = prompt::stack(target);
    crate::trace::job("toolchain_resolver", &instructions, &input);
    let turn = openai
        .respond(&instructions, &input, &registry.schemas())
        .await?;
    if turn.function_calls.is_empty() {
        return Err(DreamError::composer(
            "toolchain was not declared; call set_toolchain",
        ));
    }

    let mut toolchain = None;
    for call in turn.function_calls {
        let mut ctx = ToolCtx::resolve(project, deps, &mut toolchain);
        let args = call.parsed_args()?;
        let result = registry.dispatch(&mut ctx, &call);
        if result.is_ok() {
            if let Some(name) = args.get("toolchain").and_then(serde_json::Value::as_str) {
                eprintln!("set_toolchain {name}");
            }
        }
        result?;
    }
    let Some(known) = toolchain else {
        return Err(DreamError::composer(
            "toolchain was not declared; call set_toolchain",
        ));
    };
    Ok(known)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_target_is_the_row() {
        assert_eq!(
            from_store_or_catalog("cargo", "", false).unwrap().as_str(),
            "cargo"
        );
    }

    #[test]
    fn store_reuses_a_catalog_row_for_a_hint() {
        assert_eq!(
            from_store_or_catalog("rust", "cargo", false)
                .unwrap()
                .as_str(),
            "cargo"
        );
    }

    #[test]
    fn matching_non_row_store_is_unsupported() {
        assert_eq!(
            from_store_or_catalog("cobol", "cobol", false),
            Some(Toolchain::Unsupported)
        );
    }

    #[test]
    fn unknown_target_needs_the_model() {
        assert!(from_store_or_catalog("cobol", "", false).is_none());
        assert!(from_store_or_catalog("cobol", "cobol", true).is_none());
    }
}
