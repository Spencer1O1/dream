use crate::flags::ActiveFlags;
use crate::prompt::{paragraphs, ENTRY, FOOCODE, NO_CHAT};
use crate::tools::Registry;

const GOAL: &str = "\
Your goal is to compose this Dream program as if you were implementing it for the requested target. \
Use tool calls to write a complete, hand-maintainable project with the same meaning.";

const TARGET: &str = "\
The requested target is already in the conversation. \
Use ordinary target libraries when that is how the program would be written.";

fn no_exec() -> String {
    format!("Do not execute the program. {NO_CHAT}")
}

fn preamble() -> String {
    paragraphs(&[GOAL, FOOCODE, ENTRY, TARGET, &no_exec()])
}

pub const BUILDER_PREAMBLE: &str = "\
Declare the toolchain for the project you are about to write.";

pub fn compose(registry: &Registry, flags: &ActiveFlags) -> String {
    registry.instructions(&preamble(), flags)
}

pub fn builder(registry: &Registry, flags: &ActiveFlags) -> String {
    registry.instructions(BUILDER_PREAMBLE, flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_tools_and_only_active_flags() {
        let registry = Registry::composer();
        let instructions = compose(&registry, &ActiveFlags::new(false, false));
        assert!(instructions.contains(&preamble()));
        assert!(instructions.contains(FOOCODE));
        assert!(instructions.contains(ENTRY));
        assert!(instructions.contains(NO_CHAT));
        assert!(instructions.contains("compose this Dream program"));
        assert!(instructions.contains("same meaning"));
        assert!(instructions.contains("ordinary target libraries"));
        assert!(instructions.contains(&registry.prompt_catalog()));
        assert!(instructions.contains("write_output_file"));
        assert!(instructions.contains("remove_output_file"));
        assert!(!instructions.contains("set_builder"));
        assert!(!instructions.contains("stdout"));
        assert!(!instructions.contains("--strict"));
        assert!(!instructions.contains("--no-warn"));
        assert!(compose(&registry, &ActiveFlags::new(true, false)).contains("--strict:"));
        assert!(compose(&registry, &ActiveFlags::new(false, true)).contains("--no-warn:"));
    }

    #[test]
    fn builder_prompt_is_the_pick_turn() {
        let registry = Registry::builder();
        let instructions = builder(&registry, &ActiveFlags::new(false, false));
        assert!(instructions.contains(BUILDER_PREAMBLE));
        assert!(instructions.contains("about to write"));
        assert!(!instructions.contains("just wrote"));
        assert!(instructions.contains("set_builder"));
        assert!(!instructions.contains("write_output_file"));
    }
}
