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
            .map_err(|err| DreamError::new(err.to_string()))?;
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
            }))
            .send()
            .await
            .map_err(|err| DreamError::new(format!("OpenAI request failed: {err}")))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|err| DreamError::new(format!("OpenAI response failed: {err}")))?;

        if !status.is_success() {
            let detail = serde_json::from_str::<ApiErrorBody>(&text)
                .ok()
                .and_then(|body| body.error)
                .and_then(|error| error.message)
                .unwrap_or(text);
            return Err(DreamError::new(format!(
                "OpenAI request failed ({status}): {detail}"
            )));
        }

        let parsed: ResponsesBody = serde_json::from_str(&text).map_err(|err| {
            DreamError::new(format!("OpenAI returned an unexpected response: {err}"))
        })?;

        if let Some(message) = parsed
            .error
            .and_then(|error| error.message)
            .filter(|message| !message.is_empty())
        {
            return Err(DreamError::new(format!("OpenAI request failed: {message}")));
        }

        if let Some(status) = parsed.status.as_deref() {
            if status == "failed" || status == "cancelled" {
                return Err(DreamError::new(format!(
                    "OpenAI response {status} before the program settled"
                )));
            }
        }

        let output = parsed.output.unwrap_or_default();
        let function_calls = output
            .iter()
            .filter(|item| item["type"] == "function_call")
            .map(parse_function_call)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ResponseTurn {
            output,
            function_calls,
        })
    }
}

fn parse_function_call(item: &Value) -> Result<FunctionCall, DreamError> {
    let call_id = item["call_id"]
        .as_str()
        .ok_or_else(|| DreamError::new("OpenAI function call is missing call_id"))?;
    let name = item["name"]
        .as_str()
        .ok_or_else(|| DreamError::new("OpenAI function call is missing name"))?;
    let arguments = match &item["arguments"] {
        Value::String(value) => value.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    };
    Ok(FunctionCall {
        call_id: call_id.to_string(),
        name: name.to_string(),
        arguments,
    })
}
