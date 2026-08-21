use crate::flags::ActiveFlags;
use crate::prompt::{paragraphs, ENTRY, FOOCODE, NO_CHAT};
use crate::toolchain::Toolchain;
use crate::tools::Registry;

const GOAL: &str = "\
Your goal is to compose this Dream program as if you were implementing it for the requested target. \
Write a complete, hand-maintainable project with the same meaning.";

const REPAIR: &str = "\
Your goal is to rewrite dest files so the build succeeds. \
Write this toolchain's setup files if that is what the diagnostics need.";

const TARGET: &str = "\
The requested target is already in the conversation. \
Use ordinary target libraries when that is how the program would be written.";

const DEST_SETUP: &str = "\
Write the setup files listed for this toolchain. \
Pass the entry `.foo` file as unit on those writes. \
Do not write paths listed as project. \
Every other dest file names the `.foo` file that owns it. \
Read a non-entry `.foo` file before it can own a dest file.";

const DEST_NO_SETUP: &str = "\
This toolchain has no setup files. \
Every dest file names the `.foo` file that owns it. \
Read a non-entry `.foo` file before it can own a dest file.";

const REPAIR_DEST_SETUP: &str = "\
Overwrite dest files this run already owns. \
Write this toolchain's setup files if the diagnostics need them. \
Do not create other new dest files.";

const REPAIR_DEST_NO_SETUP: &str = "\
Overwrite dest files this run already owns. \
Do not create other new dest files.";

const LOCKED: &str = "\
A locked `.foo` file must not be written.";

fn dest_rules(toolchain: Option<Toolchain>) -> &'static str {
    if toolchain
        .and_then(Toolchain::spec)
        .is_some_and(|spec| !spec.setup.is_empty())
    {
        DEST_SETUP
    } else {
        DEST_NO_SETUP
    }
}

fn repair_dest_rules(toolchain: Option<Toolchain>) -> &'static str {
    if toolchain
        .and_then(Toolchain::spec)
        .is_some_and(|spec| !spec.setup.is_empty())
    {
        REPAIR_DEST_SETUP
    } else {
        REPAIR_DEST_NO_SETUP
    }
}

fn preamble(toolchain: Option<Toolchain>) -> String {
    paragraphs(&[
        GOAL,
        FOOCODE,
        ENTRY,
        TARGET,
        dest_rules(toolchain),
        LOCKED,
        NO_CHAT,
    ])
}

fn repair_preamble(toolchain: Option<Toolchain>) -> String {
    paragraphs(&[
        REPAIR,
        FOOCODE,
        ENTRY,
        TARGET,
        repair_dest_rules(toolchain),
        LOCKED,
        NO_CHAT,
    ])
}

pub const TOOLCHAIN_PREAMBLE: &str = "\
Pick the catalog row that builds this target, or unsupported.";

pub fn compose(registry: &Registry, flags: &ActiveFlags, toolchain: Option<Toolchain>) -> String {
    registry.instructions(&preamble(toolchain), flags)
}

pub fn repair(registry: &Registry, flags: &ActiveFlags, toolchain: Option<Toolchain>) -> String {
    registry.instructions(&repair_preamble(toolchain), flags)
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
        let cargo = Some(Toolchain::parse("cargo").unwrap());
        let instructions = compose(&registry, &ActiveFlags::new(false), cargo);
        assert!(instructions.contains(&preamble(cargo)));
        assert!(instructions.contains(&registry.prompt_catalog()));
        assert!(instructions.contains("write_file"));
        assert!(instructions.contains("remove_file"));
        assert!(instructions.contains("read_file"));
        assert!(instructions.contains("setup files listed"));
        assert!(instructions.contains("locked `.foo` file"));
        assert!(!instructions.contains("set_toolchain"));
        assert!(!instructions.contains("stdout"));
        assert!(!instructions.contains("--strict"));
        assert!(!instructions.contains("--no-warn"));
        assert!(!instructions.contains("dependencies"));
        assert!(compose(&registry, &ActiveFlags::new(true), cargo).contains("--strict:"));
        assert!(!compose(&registry, &ActiveFlags::new(true), cargo).contains("--no-warn"));
        let lua = Some(Toolchain::parse("lua").unwrap());
        let lua_prompt = compose(&registry, &ActiveFlags::new(false), lua);
        assert!(lua_prompt.contains("has no setup files"));
        assert!(!lua_prompt.contains("Write the setup files listed"));
    }

    #[test]
    fn toolchain_prompt_is_the_pick_turn() {
        let registry = Registry::toolchain();
        let instructions = toolchain(&registry);
        assert!(instructions.contains(TOOLCHAIN_PREAMBLE));
        assert!(instructions.contains("set_toolchain"));
        assert!(!instructions.contains("write_file"));
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
        let cargo = Some(Toolchain::parse("cargo").unwrap());
        let instructions = repair(&registry, &ActiveFlags::new(false), cargo);
        assert!(instructions.contains(&repair_preamble(cargo)));
        assert!(!instructions.contains(&preamble(cargo)));
        assert!(instructions.contains("write_file"));
        assert!(instructions.contains("read_file"));
        assert!(instructions.contains("setup files"));
        assert!(instructions.contains("locked `.foo` file"));
        assert!(!instructions.contains("Every other dest file names"));
        assert!(!instructions.contains("existing output"));
        assert!(!instructions.contains("remove_file"));
        assert!(!instructions.contains("set_dependencies"));
        assert!(!instructions.contains("set_toolchain"));
        assert!(!instructions.contains("stdout"));
        assert!(!instructions.contains("--strict"));
        assert!(!instructions.contains("--no-warn"));
        assert!(repair(&registry, &ActiveFlags::new(true), cargo).contains("--strict:"));
    }
}
