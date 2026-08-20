use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};

use crate::error::DreamError;

use super::call::ResponseTurn;
use super::parse::{turn_from_body, ApiErrorBody, ResponsesBody};

const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";

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
