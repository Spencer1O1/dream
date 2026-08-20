use crate::flags::ActiveFlags;
use crate::prompt::{paragraphs, ENTRY, FOOCODE, NO_CHAT};
use crate::tools::Registry;

const GOAL: &str = "\
Your goal is to execute this Dream program as if it were actually running. \
Use tool calls to do that, in the order the running program would.";

fn preamble() -> String {
    paragraphs(&[GOAL, FOOCODE, ENTRY, NO_CHAT])
}

pub fn compose(registry: &Registry, flags: &ActiveFlags) -> String {
    registry.instructions(&preamble(), flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_tools_and_only_active_flags() {
        let registry = Registry::interpreter();
        let instructions = compose(&registry, &ActiveFlags::new(false));
        assert!(instructions.contains(&preamble()));
        assert!(instructions.contains(FOOCODE));
        assert!(instructions.contains(ENTRY));
        assert!(instructions.contains(NO_CHAT));
        assert!(instructions.contains(&registry.prompt_catalog()));
        assert!(!instructions.contains("--strict"));
        assert!(compose(&registry, &ActiveFlags::new(true)).contains("--strict:"));
    }
}
