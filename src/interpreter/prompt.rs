//! Lucid instructions.
//!
//! No source-file law, setup, locks, or toolchains.

use crate::flags::ActiveFlags;
use crate::prompt::{paragraphs, ENTRY, FOOCODE, NO_CHAT};
use crate::tools::Registry;

const GOAL: &str = "\
Your goal is to execute this Dream program as if it were actually running, \
in the order the running program would.";

fn preamble() -> String {
    paragraphs(&[FOOCODE, ENTRY, GOAL, NO_CHAT])
}

pub fn lucid(registry: &Registry, flags: &ActiveFlags) -> String {
    registry.instructions(&preamble(), flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_tools_and_only_active_flags() {
        let registry = Registry::interpreter();
        let instructions = lucid(&registry, &ActiveFlags::new(false));
        let law = preamble();
        assert!(law.starts_with(FOOCODE));
        assert!(law.find(ENTRY).unwrap() < law.find(GOAL).unwrap());
        assert!(instructions.contains(&law));
        assert!(!lucid(&registry, &ActiveFlags::new(true)).contains("\n\n\n"));
        assert!(instructions.contains(&registry.tool_list()));
        assert!(!instructions.contains("--strict"));
        assert!(!instructions.contains("--no-warn"));
        assert!(lucid(&registry, &ActiveFlags::new(true)).contains("--strict:"));
    }
}
