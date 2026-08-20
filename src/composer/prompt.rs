use crate::flags::ActiveFlags;
use crate::prompt::{paragraphs, ENTRY, FOOCODE, NO_CHAT};
use crate::tools::Registry;

const GOAL: &str = "\
Your goal is to compose this Dream program as if you were implementing it for the requested target. \
Write a complete, hand-maintainable project with the same meaning.";

const REPAIR: &str = "\
Your goal is to rewrite existing output files so the build succeeds.";

const TARGET: &str = "\
The requested target is already in the conversation. \
Use ordinary target libraries when that is how the program would be written.";

const PROJECT: &str = "\
Files at project-owned paths from the toolchain must not be modified.";

const LOCKED: &str = "\
Owned files and dependencies of a locked `.foo` file must not be modified.";

fn preamble() -> String {
    paragraphs(&[GOAL, FOOCODE, ENTRY, TARGET, PROJECT, LOCKED, NO_CHAT])
}

fn repair_preamble() -> String {
    paragraphs(&[REPAIR, FOOCODE, ENTRY, TARGET, PROJECT, LOCKED, NO_CHAT])
}

pub const TOOLCHAIN_PREAMBLE: &str = "\
Declare the toolchain for the project you are about to write.";

pub fn compose(registry: &Registry, flags: &ActiveFlags) -> String {
    registry.instructions(&preamble(), flags)
}

pub fn repair(registry: &Registry, flags: &ActiveFlags) -> String {
    registry.instructions(&repair_preamble(), flags)
}

pub fn toolchain(registry: &Registry) -> String {
    format!("{TOOLCHAIN_PREAMBLE}\n\n{}", registry.tool_list())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_tools_and_only_active_flags() {
        let registry = Registry::composer();
        let instructions = compose(&registry, &ActiveFlags::new(false));
        assert!(instructions.contains(&preamble()));
        assert!(instructions.contains(&registry.prompt_catalog()));
        assert!(instructions.contains("write_output_file"));
        assert!(instructions.contains("remove_output_file"));
        assert!(instructions.contains("project-owned paths"));
        assert!(instructions.contains("locked `.foo` file"));
        assert!(!instructions.contains("set_toolchain"));
        assert!(!instructions.contains("stdout"));
        assert!(!instructions.contains("--strict"));
        assert!(!instructions.contains("--no-warn"));
        assert!(compose(&registry, &ActiveFlags::new(true)).contains("--strict:"));
        assert!(!compose(&registry, &ActiveFlags::new(true)).contains("--no-warn"));
    }

    #[test]
    fn toolchain_prompt_is_the_pick_turn() {
        let registry = Registry::toolchain();
        let instructions = toolchain(&registry);
        assert!(instructions.contains(TOOLCHAIN_PREAMBLE));
        assert!(instructions.contains("set_toolchain"));
        assert!(!instructions.contains("write_output_file"));
        assert!(!instructions.contains("dream_error"));
        assert!(!instructions.contains("entire interface"));
        assert!(!instructions.contains("--strict"));
        assert!(!instructions.contains("--no-warn"));
        assert!(!instructions.contains("Running with flags"));
        assert!(!instructions.contains("project-owned"));
        assert!(!instructions.contains("locked"));
    }

    #[test]
    fn repair_is_overwrite_not_compose() {
        let registry = Registry::repair();
        let instructions = repair(&registry, &ActiveFlags::new(false));
        assert!(instructions.contains(&repair_preamble()));
        assert!(!instructions.contains(&preamble()));
        assert!(instructions.contains("write_output_file"));
        assert!(instructions.contains("project-owned paths"));
        assert!(instructions.contains("locked `.foo` file"));
        assert!(!instructions.contains("remove_output_file"));
        assert!(!instructions.contains("set_dependencies"));
        assert!(!instructions.contains("set_toolchain"));
        assert!(!instructions.contains("stdout"));
        assert!(!instructions.contains("--strict"));
        assert!(!instructions.contains("--no-warn"));
        assert!(repair(&registry, &ActiveFlags::new(true)).contains("--strict:"));
    }
}
