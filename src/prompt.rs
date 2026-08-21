//! Shared by interpreter and composer. Mode-specific goals stay in those prompts.

pub const FOOCODE: &str = "\
A Dream program is foocode: informal notation in `.foo` files. \
One `.foo` file is one semantic unit. There is no grammar and no keywords.";

pub const ENTRY: &str = "\
The entry unit is already in the conversation.";

pub const NO_CHAT: &str = "Chat text is discarded. Do not chat.";

pub fn paragraphs(parts: &[&str]) -> String {
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_are_blank_line_separated() {
        assert_eq!(paragraphs(&["a", "b"]), "a\n\nb");
    }
}
