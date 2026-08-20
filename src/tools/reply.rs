use serde_json::{json, Value};

use crate::error::DreamError;

pub fn warning(message: impl Into<String>) -> String {
    json!({ "ok": false, "warning": message.into() }).to_string()
}

pub fn refused(err: DreamError) -> String {
    warning(err.detail())
}

pub fn warning_of(output: &str) -> Option<String> {
    let value: Value = serde_json::from_str(output).ok()?;
    if value.get("ok") != Some(&json!(false)) {
        return None;
    }
    value
        .get("warning")
        .and_then(Value::as_str)
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warning_round_trips() {
        let out = warning("`utils.foo` is locked");
        assert_eq!(warning_of(&out).as_deref(), Some("`utils.foo` is locked"));
        assert_eq!(warning_of(r#"{"ok":true,"path":"x"}"#), None);
    }
}
