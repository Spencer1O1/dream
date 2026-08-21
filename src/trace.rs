//! Operator dump of instructions and user cards.
//!
//! Set `DREAM_TRACE=1`. Not a product flag. Not model-facing.
//! Prints once per job, then any later user card (the missing-units nudge).
//! Does not dump tool results or the growing transcript.

use serde_json::Value;

pub fn job(kind: &str, instructions: &str, input: &[Value]) {
    if !enabled() {
        return;
    }
    eprint!("{}", render(kind, instructions, input));
}

pub fn user(content: &str) {
    if !enabled() {
        return;
    }
    eprint!("{}", user_block(content));
}

fn enabled() -> bool {
    parse(&std::env::var("DREAM_TRACE").ok())
}

fn parse(raw: &Option<String>) -> bool {
    matches!(
        raw.as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

fn render(kind: &str, instructions: &str, input: &[Value]) -> String {
    let mut out = format!("=== instructions ({kind}) ===\n{instructions}\n\n");
    for item in input {
        if item.get("role").and_then(Value::as_str) != Some("user") {
            continue;
        }
        if let Some(content) = item.get("content").and_then(Value::as_str) {
            out.push_str(&user_block(content));
        }
    }
    out
}

fn user_block(content: &str) -> String {
    format!("=== user ===\n{content}\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_is_opt_in() {
        assert!(!parse(&None));
        assert!(!parse(&Some(String::new())));
        assert!(!parse(&Some("0".into())));
        assert!(!parse(&Some("false".into())));
        assert!(parse(&Some("1".into())));
        assert!(parse(&Some("YES".into())));
        assert!(parse(&Some(" true ".into())));
    }

    #[test]
    fn render_is_instructions_then_user_cards() {
        let input = vec![
            json!({"role": "user", "content": "Entry `.foo` file: limits.foo\n\nprint"}),
            json!({"role": "user", "content": "Target toolchain: cargo\n\n{}"}),
            json!({"type": "function_call_output", "output": "secret"}),
        ];
        let dump = render("compose", "Write source files.", &input);
        assert!(dump.starts_with("=== instructions (compose) ===\nWrite source files.\n\n"));
        assert!(dump.contains("=== user ===\nEntry `.foo` file: limits.foo\n\nprint\n\n"));
        assert!(dump.contains("=== user ===\nTarget toolchain: cargo\n\n{}\n\n"));
        assert!(!dump.contains("secret"));
        assert!(!dump.contains("function_call_output"));
    }
}
