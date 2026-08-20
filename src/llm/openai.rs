use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::DreamError;

const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

impl FunctionCall {
    pub fn parsed_args(&self) -> Result<Value, DreamError> {
        if self.arguments.trim().is_empty() {
            Ok(json!({}))
        } else {
            serde_json::from_str(&self.arguments).map_err(|_| {
                DreamError::runtime(format!("invalid arguments for tool `{}`", self.name))
            })
        }
    }
}

#[derive(Debug)]
pub struct ResponseTurn {
    pub output: Vec<Value>,
    pub function_calls: Vec<FunctionCall>,
}

#[derive(Debug, Deserialize)]
struct ResponsesBody {
    status: Option<String>,
    error: Option<ApiErrorMessage>,
    output: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: Option<ApiErrorMessage>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorMessage {
    message: Option<String>,
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

pub struct OpenAi {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAi {
    pub fn new(api_key: String, model: String) -> Result<Self, DreamError> {
        let client = reqwest::Client::builder()
            .user_agent("dream/0.1")
            .build()
            .map_err(|err| DreamError::runtime(err.to_string()))?;
        Ok(Self {
            client,
            api_key,
            model,
        })
    }

    pub async fn respond(
        &self,
        instructions: &str,
        input: &[Value],
        tools: &[Value],
    ) -> Result<ResponseTurn, DreamError> {
        let response = self
            .client
            .post(RESPONSES_URL)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(CONTENT_TYPE, "application/json")
            .json(&json!({
                "model": self.model,
                "instructions": instructions,
                "input": input,
                "tools": tools,
                "parallel_tool_calls": true,
            }))
            .send()
            .await
            .map_err(|err| DreamError::runtime(format!("OpenAI request failed: {err}")))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|err| DreamError::runtime(format!("OpenAI response failed: {err}")))?;

        if !status.is_success() {
            let detail = serde_json::from_str::<ApiErrorBody>(&text)
                .ok()
                .and_then(|body| body.error)
                .and_then(|error| error.message)
                .unwrap_or(text);
            return Err(DreamError::runtime(format!(
                "OpenAI request failed ({status}): {detail}"
            )));
        }

        let parsed: ResponsesBody = serde_json::from_str(&text).map_err(|err| {
            DreamError::runtime(format!("OpenAI returned an unexpected response: {err}"))
        })?;

        turn_from_body(parsed)
    }
}

fn turn_from_body(parsed: ResponsesBody) -> Result<ResponseTurn, DreamError> {
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
