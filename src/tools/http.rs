//! Catalog tool. Description says what it does; parameters say what to write. No Dream law.

use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{json, Value};

use crate::error::DreamError;

use crate::tools::Mode;

use super::{
    arg_str, enum_arg, nullable_string_arg, object_array_arg, object_params, string_arg, Family,
    Tool, ToolCtx, ToolSpec,
};

const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"];
const MAX_BODY: usize = 1_000_000;

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(HttpRequest)]
}

struct HttpRequest;

impl Tool for HttpRequest {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "http_request",
            family: Family::Runtime,
            description:
                "Send an HTTP request. Dream performs it. Returns status, headers, and body.",
            parameters: object_params(
                &[
                    ("method", enum_arg("HTTP method", METHODS)),
                    ("url", string_arg("Absolute http or https URL")),
                    ("body", nullable_string_arg("Request body, or null if none")),
                    (
                        "headers",
                        object_array_arg(
                            "Request headers",
                            &[
                                ("name", string_arg("Header name")),
                                ("value", string_arg("Header value")),
                            ],
                            &["name", "value"],
                        ),
                    ),
                ],
                &["method", "url", "body", "headers"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        if !matches!(ctx.mode, Mode::Lucid) {
            return Err(DreamError::runtime(
                "http_request is only available when interpreting",
            ));
        }
        Ok(send(args)?.to_string())
    }
}

fn send(args: &Value) -> Result<Value, DreamError> {
    let method = arg_str(args, "method");
    let url = arg_str(args, "url");
    if url.is_empty() {
        return Err(DreamError::runtime("url is empty"));
    }
    let parsed = reqwest::Url::parse(url).map_err(|err| DreamError::runtime(err.to_string()))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(DreamError::runtime("url must be http or https"));
    }
    let body = match &args["body"] {
        Value::Null => None,
        Value::String(text) => Some(text.as_str()),
        _ => None,
    };
    let headers = headers(args)?;
    let client = Client::builder()
        .user_agent("dream/0.1")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| DreamError::runtime(err.to_string()))?;
    let mut request = client.request(
        method
            .parse()
            .map_err(|_| DreamError::runtime(format!("unknown method `{method}`")))?,
        parsed,
    );
    request = request.headers(headers);
    if let Some(body) = body {
        request = request.body(body.to_string());
    }
    let response = match request.send() {
        Ok(response) => response,
        Err(err) => return Ok(json!({ "ok": false, "error": err.to_string() })),
    };
    let status = response.status().as_u16();
    let reply_headers: Vec<Value> = response
        .headers()
        .iter()
        .map(|(name, value)| {
            json!({
                "name": name.as_str(),
                "value": value.to_str().unwrap_or(""),
            })
        })
        .collect();
    let bytes = match response.bytes() {
        Ok(bytes) => bytes,
        Err(err) => return Ok(json!({ "ok": false, "error": err.to_string() })),
    };
    if bytes.len() > MAX_BODY {
        return Ok(json!({ "ok": false, "error": "response body is too large" }));
    }
    let body = match String::from_utf8(bytes.to_vec()) {
        Ok(body) => body,
        Err(_) => return Ok(json!({ "ok": false, "error": "response is not UTF-8" })),
    };
    Ok(json!({
        "ok": true,
        "status": status,
        "headers": reply_headers,
        "body": body,
    }))
}

fn headers(args: &Value) -> Result<HeaderMap, DreamError> {
    let mut map = HeaderMap::new();
    let Some(items) = args["headers"].as_array() else {
        return Ok(map);
    };
    for item in items {
        let name = arg_str(item, "name");
        let value = arg_str(item, "value");
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| DreamError::runtime(format!("invalid header name `{name}`")))?;
        let value = HeaderValue::from_str(value)
            .map_err(|_| DreamError::runtime(format!("invalid header value for `{name}`")))?;
        map.append(name, value);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{DepGraph, Project};
    use crate::tools::ToolCtx;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn rejects_non_http_urls() {
        let err = send(&json!({
            "method": "GET",
            "url": "file:///etc/passwd",
            "body": null,
            "headers": []
        }))
        .unwrap_err();
        assert!(err.to_string().contains("http or https"));
    }

    #[test]
    fn get_returns_status_and_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                .unwrap();
        });
        let out = send(&json!({
            "method": "GET",
            "url": format!("http://{addr}/"),
            "body": null,
            "headers": []
        }))
        .unwrap();
        server.join().unwrap();
        assert_eq!(out["ok"], true);
        assert_eq!(out["status"], 200);
        assert_eq!(out["body"], "ok");
    }

    #[test]
    fn only_lucid() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.foo"), "entry").unwrap();
        let (project, unit) = Project::from_entry(&dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let mut toolchain = None;
        let mut ctx = ToolCtx::pick(&project, &mut deps, &mut toolchain);
        let err = HttpRequest
            .call(
                &mut ctx,
                &json!({
                    "method": "GET",
                    "url": "http://127.0.0.1/",
                    "body": null,
                    "headers": []
                }),
            )
            .unwrap_err();
        assert!(err.to_string().contains("interpreting"));
    }
}
