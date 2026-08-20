use crate::flags::ActiveFlags;
use crate::tools::Registry;

pub const PREAMBLE: &str = "\
Your goal is to compose this Dream program as if you were implementing it for the requested target. \
Use tool calls to write a complete, hand-maintainable project with the same meaning.

A Dream program is foocode: informal notation in .foo files. \
One .foo file is one semantic unit. There is no grammar and no keywords.

The entry unit is already in the conversation. \
Request other source units instead of inventing them.

The requested target is already in the conversation.

Do not execute the program. Chat text is discarded. Do not chat.";

pub const BUILDER_PREAMBLE: &str = "\
Declare the toolchain for the project you just wrote.";

pub fn compose(registry: &Registry, flags: &ActiveFlags) -> String {
    registry.instructions(PREAMBLE, flags)
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
        let instructions = compose(&registry, &ActiveFlags::new(false));
        assert!(instructions.contains(PREAMBLE));
        assert!(instructions.contains("compose this Dream program"));
        assert!(instructions.contains("same meaning"));
        assert!(instructions.contains(&registry.prompt_catalog()));
        assert!(instructions.contains("write_output_file"));
        assert!(instructions.contains("remove_output_file"));
        assert!(!instructions.contains("set_builder"));
        assert!(!instructions.contains("stdout"));
        assert!(!instructions.contains("--strict"));
        assert!(compose(&registry, &ActiveFlags::new(true)).contains("--strict:"));
    }

    #[test]
    fn builder_prompt_is_the_pick_turn() {
        let registry = Registry::builder();
        let instructions = builder(&registry, &ActiveFlags::new(false));
        assert!(instructions.contains(BUILDER_PREAMBLE));
        assert!(instructions.contains("set_builder"));
        assert!(!instructions.contains("write_output_file"));
    }
}
