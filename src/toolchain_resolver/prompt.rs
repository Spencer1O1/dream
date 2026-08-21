//! Toolchain resolver instructions.
//!
//! No standing law. The stack is only the requested target.

use serde_json::{json, Value};

use crate::prompt::paragraphs;
use crate::tools::Registry;

pub fn instructions(registry: &Registry) -> String {
    paragraphs(&[
        "Choose the toolchain for the project",
        &registry.tool_list(),
    ])
}

/// Resolver sees only the `-t` target. Not the entry `.foo` file.
pub fn stack(target: &str) -> Vec<Value> {
    vec![json!({
        "role": "user",
        "content": crate::prompt::requested_target(target),
    })]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_is_only_the_target() {
        let stack = stack("cobol");
        assert_eq!(stack.len(), 1);
        assert_eq!(
            stack[0]["content"].as_str().unwrap(),
            "Requested target: cobol"
        );
    }

    #[test]
    fn instructions_are_the_resolver_turn() {
        let registry = Registry::toolchain();
        let instructions = instructions(&registry);
        assert!(instructions.contains(&registry.tool_list()));
        assert!(!instructions.contains("write_source_file"));
    }
}
