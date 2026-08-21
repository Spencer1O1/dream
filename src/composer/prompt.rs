//! Compose and repair instructions.
//!
//! Standing law for those modes, plus the this-run user stack
//! (entry card, optional toolchain card).
//!
//! No diagnostics, no write loop, no tool descriptions.

use serde_json::{json, Value};

use crate::error::DreamError;
use crate::flags::ActiveFlags;
use crate::prompt::{paragraphs, ENTRY, FOOCODE, NO_CHAT};
use crate::toolchain::Toolchain;
use crate::tools::Registry;

const COMPOSE: &str = "\
Your goal is to compose this Dream program, implementing it for the requested target.";

const REPAIR: &str = "\
Your goal is to repair this Dream program, implementing it for the requested target.";

const WRITE: &str = "\
Write the source files of each `.foo` file you read.";

const NO_MIX: &str = "\
Do not mix source from different `.foo` files in the same source file.";

const READ_FIRST: &str = "\
Always read a non-entry `.foo` file before writing the source files it produces.";

const LIBRARIES: &str = "\
Use ordinary libraries when that is how the program would be written.";

const SETUP: &str = "\
Write the required setup files for the toolchain.";

const LOCKED: &str = "\
Do not write or remove locked source files (source produced by a locked `.foo` file) or locked setup files.";

fn setup_rule(toolchain: Option<Toolchain>) -> &'static str {
    if toolchain
        .and_then(Toolchain::spec)
        .is_some_and(|spec| !spec.setup.is_empty())
    {
        SETUP
    } else {
        ""
    }
}

fn preamble(goal: &str, toolchain: Option<Toolchain>) -> String {
    paragraphs(&[
        FOOCODE,
        ENTRY,
        goal,
        WRITE,
        NO_MIX,
        READ_FIRST,
        LIBRARIES,
        setup_rule(toolchain),
        LOCKED,
        NO_CHAT,
    ])
}

pub fn compose(registry: &Registry, flags: &ActiveFlags, toolchain: Option<Toolchain>) -> String {
    registry.instructions(&preamble(COMPOSE, toolchain), flags)
}

pub fn repair(registry: &Registry, flags: &ActiveFlags, toolchain: Option<Toolchain>) -> String {
    registry.instructions(&preamble(REPAIR, toolchain), flags)
}

pub fn this_run(
    entry_rel: &str,
    source: &str,
    toolchain: Option<Toolchain>,
) -> Result<Vec<Value>, DreamError> {
    this_run_for(entry_rel, source, toolchain, None)
}

pub fn this_run_for(
    entry_rel: &str,
    source: &str,
    toolchain: Option<Toolchain>,
    requested: Option<&str>,
) -> Result<Vec<Value>, DreamError> {
    let mut input = vec![json!({
        "role": "user",
        "content": crate::prompt::entry(entry_rel, source),
    })];
    if let Some(known) = toolchain {
        push_bind(
            &mut input,
            known,
            requested.unwrap_or(known.as_str()),
            entry_rel,
        )?;
    }
    Ok(input)
}

/// Catalog row → toolchain card. No row → requested-target card. Never `Toolchain unsupported`.
pub fn push_bind(
    input: &mut Vec<Value>,
    toolchain: Toolchain,
    requested: &str,
    entry_rel: &str,
) -> Result<(), DreamError> {
    if toolchain.spec().is_some() {
        push_toolchain(input, toolchain, entry_rel)
    } else {
        push_requested(input, requested);
        Ok(())
    }
}

pub fn store_name(toolchain: Toolchain, requested: &str) -> String {
    if toolchain.spec().is_some() {
        toolchain.as_str().to_string()
    } else {
        requested.to_string()
    }
}

pub fn push_toolchain(
    input: &mut Vec<Value>,
    toolchain: Toolchain,
    entry_rel: &str,
) -> Result<(), DreamError> {
    input.push(json!({
        "role": "user",
        "content": toolchain.declared_user_blob(entry_rel)?,
    }));
    Ok(())
}

pub fn push_requested(input: &mut Vec<Value>, hint: &str) {
    input.push(json!({
        "role": "user",
        "content": crate::prompt::requested_target(hint),
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_assembles_the_compose_preamble_and_tools() {
        let registry = Registry::composer();
        let cargo = Some(Toolchain::parse("cargo").unwrap());
        let instructions = compose(&registry, &ActiveFlags::new(false), cargo);
        let law = preamble(COMPOSE, cargo);
        assert!(law.starts_with(FOOCODE));
        assert!(law.find(ENTRY).unwrap() < law.find(COMPOSE).unwrap());
        assert!(instructions.contains(&law));
        assert!(instructions.contains(&registry.tool_list()));
        assert!(!instructions.contains("set_toolchain"));
        assert!(!instructions.contains("stdout"));
        assert!(!compose(&registry, &ActiveFlags::new(false), cargo).contains("--strict:"));
        let strict = compose(&registry, &ActiveFlags::new(true), cargo);
        assert!(strict.contains("--strict:"));
        assert!(!strict.contains("--no-warn"));
        assert!(strict.find("Running with flags:").unwrap() < strict.find("Tools:").unwrap());
        assert!(!strict.contains("\n\n\n"));
    }

    #[test]
    fn setup_rule_follows_the_toolchain() {
        let cargo = Some(Toolchain::parse("cargo").unwrap());
        let lua = Some(Toolchain::parse("lua").unwrap());
        assert!(preamble(COMPOSE, cargo).contains(SETUP));
        assert!(!preamble(COMPOSE, lua).contains(SETUP));
        assert!(preamble(COMPOSE, cargo).contains(READ_FIRST));
        assert!(preamble(COMPOSE, lua).contains(READ_FIRST));
    }

    #[test]
    fn this_run_is_entry_then_toolchain() {
        let cargo = Toolchain::parse("cargo").unwrap();
        let stack = this_run("limits.foo", "print far", Some(cargo)).unwrap();
        assert_eq!(stack.len(), 2);
        assert!(stack[0]["content"]
            .as_str()
            .unwrap()
            .starts_with("Entry `.foo` file:"));
        assert!(stack[1]["content"]
            .as_str()
            .unwrap()
            .starts_with("Toolchain cargo"));
        assert_eq!(this_run("limits.foo", "print far", None).unwrap().len(), 1);
    }

    #[test]
    fn push_requested_is_the_hint_card() {
        let mut input = this_run("limits.foo", "print far", None).unwrap();
        push_requested(&mut input, "whatever runs on arduino nano");
        assert_eq!(
            input[1]["content"].as_str().unwrap(),
            "Requested target: whatever runs on arduino nano"
        );
    }

    #[test]
    fn no_row_bind_is_the_requested_target() {
        let mut input = this_run("limits.foo", "print far", None).unwrap();
        push_bind(&mut input, Toolchain::Unsupported, "monkey_c", "limits.foo").unwrap();
        assert_eq!(
            input[1]["content"].as_str().unwrap(),
            "Requested target: monkey_c"
        );
        assert!(!input[1]["content"]
            .as_str()
            .unwrap()
            .contains("Toolchain unsupported"));
        assert_eq!(store_name(Toolchain::Unsupported, "monkey_c"), "monkey_c");
        assert_eq!(
            store_name(Toolchain::parse("cargo").unwrap(), "rust"),
            "cargo"
        );
    }

    #[test]
    fn repair_assembles_the_repair_preamble_and_tools() {
        let registry = Registry::repair();
        let cargo = Some(Toolchain::parse("cargo").unwrap());
        let instructions = repair(&registry, &ActiveFlags::new(false), cargo);
        assert!(instructions.contains(&preamble(REPAIR, cargo)));
        assert!(instructions.contains(&registry.tool_list()));
        assert!(!instructions.contains(COMPOSE));
        assert!(!compose(&Registry::composer(), &ActiveFlags::new(false), cargo).contains(REPAIR));
        assert!(repair(&registry, &ActiveFlags::new(true), cargo).contains("--strict:"));
    }
}
