use serde_json::Value;

use crate::error::DreamError;

pub fn tool(name: &str, args: &Value) {
    eprintln!("{}", line(name, args));
}

pub fn rejected(name: &str, args: &Value, err: &DreamError) {
    eprintln!("{}: {}", line(name, args), err.detail());
}

pub fn warning(name: &str, args: &Value, message: &str) {
    eprintln!("warning: {}: {message}", line(name, args));
}

pub fn repair() {
    eprintln!("repair");
}

fn line(name: &str, args: &Value) -> String {
    for key in ["path", "toolchain", "unit"] {
        if let Some(value) = args
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            return format!("{name} {value}");
        }
    }
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn line_is_name_and_path_or_toolchain() {
        assert_eq!(
            line("write_output_file", &json!({"path": "src/main.rs"})),
            "write_output_file src/main.rs"
        );
        assert_eq!(
            line("set_toolchain", &json!({"toolchain": "cargo"})),
            "set_toolchain cargo"
        );
        assert_eq!(
            line("set_dependencies", &json!({"unit": "main.foo"})),
            "set_dependencies main.foo"
        );
        assert_eq!(line("list_source_files", &json!({})), "list_source_files");
        assert_eq!(
            line("dream_error", &json!({"error": "nope"})),
            "dream_error"
        );
    }

    #[test]
    fn rejected_uses_the_detail_not_the_subtype_prefix() {
        let err = DreamError::composer("`utils.foo` is locked");
        assert_eq!(
            format!(
                "{}: {}",
                line("write_output_file", &json!({"path": "src/utils.rs"})),
                err.detail()
            ),
            "write_output_file src/utils.rs: `utils.foo` is locked"
        );
        assert_eq!(
            format!(
                "warning: {}: {}",
                line("write_output_file", &json!({"path": "src/utils.rs"})),
                "`utils.foo` is locked"
            ),
            "warning: write_output_file src/utils.rs: `utils.foo` is locked"
        );
    }
}
