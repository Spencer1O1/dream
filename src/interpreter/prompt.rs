use crate::flags::ActiveFlags;
use crate::tools::Registry;

pub const PREAMBLE: &str = "\
Your goal is to execute this Dream program as if it were actually running. \
Use tool calls to do that, in the order the running program would.

A Dream program is foocode: informal notation in .foo files. \
One .foo file is one semantic unit. There is no grammar and no keywords.

The entry unit is already in the conversation. \
Request other source units instead of inventing them.

Chat text is discarded. Do not chat.";

pub fn compose(registry: &Registry, flags: &ActiveFlags) -> String {
    registry.instructions(PREAMBLE, flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_tools_and_only_active_flags() {
        let registry = Registry::interpreter();
        let instructions = compose(&registry, &ActiveFlags::new(false));
        assert!(instructions.contains(PREAMBLE));
        assert!(instructions.contains(&registry.prompt_catalog()));
        assert!(!instructions.contains("--strict"));
        assert!(compose(&registry, &ActiveFlags::new(true)).contains("--strict:"));
    }
}
