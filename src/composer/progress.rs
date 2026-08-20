use serde_json::Value;

pub fn tool(name: &str, args: &Value) {
    eprintln!("{}", line(name, args));
}

pub fn repair() {
    eprintln!("repair");
}

fn line(name: &str, args: &Value) -> String {
    for key in ["path", "builder"] {
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
    fn line_is_name_and_path_or_builder() {
        assert_eq!(
            line("write_output_file", &json!({"path": "src/main.rs"})),
            "write_output_file src/main.rs"
        );
        assert_eq!(
            line("set_builder", &json!({"builder": "cargo"})),
            "set_builder cargo"
        );
        assert_eq!(line("list_source_files", &json!({})), "list_source_files");
        assert_eq!(
            line("dream_error", &json!({"error": "nope"})),
            "dream_error"
        );
    }
}
