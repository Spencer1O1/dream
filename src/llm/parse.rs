use serde::Deserialize;
use serde_json::Value;

use crate::error::DreamError;

use super::call::{FunctionCall, ResponseTurn};

#[derive(Debug, Deserialize)]
pub(super) struct ResponsesBody {
    status: Option<String>,
    error: Option<ApiErrorMessage>,
    output: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiErrorBody {
    pub error: Option<ApiErrorMessage>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiErrorMessage {
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FunctionCallItem {
    #[serde(rename = "type")]
    _kind: FunctionCallType,
    call_id: String,
    name: String,
    #[serde(default)]
    arguments: FunctionArguments,
}

#[derive(Debug, Deserialize)]
enum FunctionCallType {
    #[serde(rename = "function_call")]
    FunctionCall,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FunctionArguments {
    Text(String),
    Json(Value),
}

impl Default for FunctionArguments {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

impl FunctionArguments {
    fn into_string(self) -> String {
        match self {
            Self::Text(text) => text,
            Self::Json(Value::Null) => String::new(),
            Self::Json(value) => value.to_string(),
        }
    }
}

impl FunctionCallItem {
    fn into_call(self) -> FunctionCall {
        FunctionCall {
            call_id: self.call_id,
            name: self.name,
            arguments: self.arguments.into_string(),
        }
    }
}

pub(super) fn turn_from_body(parsed: ResponsesBody) -> Result<ResponseTurn, DreamError> {
    if let Some(message) = parsed
        .error
        .and_then(|error| error.message)
        .filter(|message| !message.is_empty())
    {
        return Err(DreamError::runtime(format!(
            "OpenAI request failed: {message}"
        )));
    }

    if let Some(status) = parsed.status.as_deref() {
        if status == "failed" || status == "cancelled" {
            return Err(DreamError::runtime(format!(
                "OpenAI response {status} before the program settled"
            )));
        }
    }

    let output = parsed.output.unwrap_or_default();
    let function_calls = output
        .iter()
        .filter_map(function_call)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ResponseTurn {
        output,
        function_calls,
    })
}

fn function_call(item: &Value) -> Option<Result<FunctionCall, DreamError>> {
    match serde_json::from_value::<FunctionCallItem>(item.clone()) {
        Ok(item) => Some(Ok(item.into_call())),
        Err(_) if item.get("type").and_then(Value::as_str) == Some("function_call") => Some(Err(
            DreamError::runtime("OpenAI function call is missing call_id or name"),
        )),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn body(output: Value) -> ResponsesBody {
        serde_json::from_value(json!({
            "status": "completed",
            "output": output
        }))
        .unwrap()
    }

    #[test]
    fn reads_typed_function_call() {
        let turn = turn_from_body(body(json!([{
            "type": "function_call",
            "id": "fc_1",
            "call_id": "call_1",
            "name": "stdout",
            "arguments": "{\"text\":\"hi\"}"
        }])))
        .unwrap();
        assert_eq!(turn.function_calls.len(), 1);
        assert_eq!(turn.function_calls[0].call_id, "call_1");
        assert_eq!(turn.function_calls[0].name, "stdout");
        assert_eq!(turn.function_calls[0].arguments, r#"{"text":"hi"}"#);
        assert_eq!(turn.output[0]["id"], "fc_1");
    }

    #[test]
    fn ignores_non_call_items() {
        let turn = turn_from_body(body(json!([{
            "type": "reasoning",
            "id": "rs_1"
        }])))
        .unwrap();
        assert!(turn.function_calls.is_empty());
        assert_eq!(turn.output[0]["type"], "reasoning");
    }

    #[test]
    fn rejects_incomplete_function_call() {
        let err = turn_from_body(body(json!([{
            "type": "function_call",
            "name": "stdout"
        }])))
        .unwrap_err();
        assert!(err.to_string().contains("call_id or name"));
    }
}
