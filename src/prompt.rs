//! Shared by interpreter and composer.
//!
//! Standing law that is true in every mode, plus shared this-run card shapes.
//! Mode-specific goals stay in those prompts.
//!
//! No compose/repair/lucid goals, no write/mix/lock/setup, no diagnostics,
//! no tool names, no turn loops.

pub const FOOCODE: &str = "\
A Dream program is foocode: informal notation in `.foo` files. \
One `.foo` file is one semantic unit. It is meant to produce source files. \
There is no grammar and no keywords.";

pub const ENTRY: &str = "\
The first user message is the entry `.foo` file.";

pub const NO_CHAT: &str = "Chat text is discarded. Do not chat.";

pub fn paragraphs(parts: &[&str]) -> String {
    parts
        .iter()
        .copied()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn entry(entry_rel: &str, source: &str) -> String {
    format!("Entry `.foo` file: {entry_rel}\n\n{source}")
}

pub fn toolchain_card(name: &str, declared: impl std::fmt::Display) -> String {
    format!("Toolchain {name}\n\n{declared}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_are_blank_line_separated() {
        assert_eq!(paragraphs(&["a", "b"]), "a\n\nb");
        assert_eq!(paragraphs(&["a", "", "b"]), "a\n\nb");
        assert_eq!(paragraphs(&["a\n", "\nb\n"]), "a\n\nb");
        assert!(!paragraphs(&["a\n\n", "b"]).contains("\n\n\n"));
    }

    #[test]
    fn entry_is_the_file_not_a_goal() {
        assert_eq!(
            entry("limits.foo", "print far origin near"),
            "Entry `.foo` file: limits.foo\n\nprint far origin near"
        );
    }

    #[test]
    fn toolchain_card_is_name_then_json() {
        assert_eq!(
            toolchain_card("go", r#"{"ok":true}"#),
            "Toolchain go\n\n{\"ok\":true}"
        );
    }
}
