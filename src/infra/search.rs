use serde_json::{Value, json};

use crate::error::Result;
use crate::infra::imagine::ImagineClient;
use crate::model::search::{WebSearchRequest, WebSearchResult, XSearchRequest, XSearchResult};

/// Official `web_search` / `x_search` tool request fields from grok-build
/// `WebSearchClient::build_request_json`.
const STORE: bool = false;
const TEMPERATURE: f64 = 0.1;
const TOP_P: f64 = 0.95;
const MAX_OUTPUT_TOKENS: u32 = 8192;

pub async fn search(client: &ImagineClient, request: &WebSearchRequest) -> Result<WebSearchResult> {
    let endpoint = format!("{}/responses", client.base_url());
    let value = client
        .post_json(&endpoint, search_request_body(request))
        .await?;
    Ok(parse_search_response(request, value))
}

pub async fn x_search(client: &ImagineClient, request: &XSearchRequest) -> Result<XSearchResult> {
    let endpoint = format!("{}/responses", client.base_url());
    let value = client
        .post_json(&endpoint, x_search_request_body(request))
        .await?;
    Ok(parse_x_search_response(request, value))
}

pub fn search_request_body(request: &WebSearchRequest) -> Value {
    responses_body(
        &request.model,
        &request.query,
        web_search_tool_entry(
            request.allowed_domains.as_deref(),
            request.excluded_domains.as_deref(),
        ),
    )
}

pub fn x_search_request_body(request: &XSearchRequest) -> Value {
    responses_body(&request.model, &request.query, x_search_tool_entry(request))
}

fn responses_body(model: &str, query: &str, tool: Value) -> Value {
    json!({
        "model": model,
        "input": query,
        "tools": [tool],
        "store": STORE,
        "temperature": TEMPERATURE,
        "top_p": TOP_P,
        "max_output_tokens": MAX_OUTPUT_TOKENS,
    })
}

/// Official wire shape from `WebSearchOptions::to_tool_entry`.
fn web_search_tool_entry(
    allowed_domains: Option<&[String]>,
    excluded_domains: Option<&[String]>,
) -> Value {
    let allowed = nonempty(allowed_domains);
    let excluded = nonempty(excluded_domains);
    let mut tool = json!({ "type": "web_search" });
    if allowed.is_some() || excluded.is_some() {
        let mut filters = json!({});
        if let Some(allowed) = allowed {
            filters["allowed_domains"] = json!(allowed);
        }
        if let Some(excluded) = excluded {
            filters["excluded_domains"] = json!(excluded);
        }
        tool["filters"] = filters;
    }
    tool
}

/// Official wire shape from grok-build `XSearchOptions::to_tool_entry`, plus
/// public Responses `x_search` keys (`allowed_x_handles`, `excluded_x_handles`,
/// `enable_image_understanding`, `enable_video_understanding`).
fn x_search_tool_entry(request: &XSearchRequest) -> Value {
    let mut tool = json!({ "type": "x_search" });
    if let Some(from_date) = nonempty_str(request.from_date.as_deref()) {
        tool["from_date"] = json!(from_date);
    }
    if let Some(to_date) = nonempty_str(request.to_date.as_deref()) {
        tool["to_date"] = json!(to_date);
    }
    if let Some(handles) = nonempty(request.allowed_x_handles.as_deref()) {
        tool["allowed_x_handles"] = json!(handles);
    }
    if let Some(handles) = nonempty(request.excluded_x_handles.as_deref()) {
        tool["excluded_x_handles"] = json!(handles);
    }
    if request.enable_image_understanding {
        tool["enable_image_understanding"] = json!(true);
    }
    if request.enable_video_understanding {
        tool["enable_video_understanding"] = json!(true);
    }
    tool
}

fn nonempty(values: Option<&[String]>) -> Option<&[String]> {
    values.filter(|values| !values.is_empty())
}

fn nonempty_str(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.is_empty())
}

pub fn parse_search_response(request: &WebSearchRequest, value: Value) -> WebSearchResult {
    let (content, citations) = parse_output(value);
    WebSearchResult {
        query: request.query.clone(),
        content,
        citations,
        allowed_domains: request.allowed_domains.clone(),
    }
}

pub fn parse_x_search_response(request: &XSearchRequest, value: Value) -> XSearchResult {
    let (content, citations) = parse_output(value);
    XSearchResult {
        query: request.query.clone(),
        content,
        citations,
        from_date: request.from_date.clone(),
        to_date: request.to_date.clone(),
        allowed_x_handles: request.allowed_x_handles.clone(),
        excluded_x_handles: request.excluded_x_handles.clone(),
    }
}

fn parse_output(value: Value) -> (String, Vec<String>) {
    (
        output_text(&value).unwrap_or_else(|| "No search results found.".to_string()),
        extract_citations(&value),
    )
}

fn output_text(value: &Value) -> Option<String> {
    let mut chunks = Vec::new();
    for item in value
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if item.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }
        for content in item
            .get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let is_text = content.get("type").and_then(Value::as_str) == Some("output_text");
            if is_text && let Some(text) = content.get("text").and_then(Value::as_str) {
                chunks.push(text);
            }
        }
    }
    if chunks.is_empty() {
        None
    } else {
        Some(chunks.concat())
    }
}

fn extract_citations(value: &Value) -> Vec<String> {
    let mut citations = Vec::new();
    for item in value
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if item.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }
        for content in item
            .get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            for annotation in content
                .get("annotations")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if annotation.get("type").and_then(Value::as_str) != Some("url_citation") {
                    continue;
                }
                if let Some(url) = annotation.get("url").and_then(Value::as_str)
                    && !url.is_empty()
                {
                    citations.push(url.to_string());
                }
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    citations.retain(|url| seen.insert(url.clone()));
    citations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::search::{WebSearchRequest, XSearchRequest};

    fn request() -> WebSearchRequest {
        WebSearchRequest {
            query: "q".into(),
            model: "grok-4.6".into(),
            allowed_domains: None,
            excluded_domains: None,
        }
    }

    fn x_request() -> XSearchRequest {
        XSearchRequest {
            query: "q".into(),
            model: "grok-4.6".into(),
            from_date: None,
            to_date: None,
            allowed_x_handles: None,
            excluded_x_handles: None,
            enable_image_understanding: false,
            enable_video_understanding: false,
        }
    }

    #[test]
    fn body_matches_official_web_search_defaults() {
        let body = search_request_body(&request());
        assert_eq!(body["model"], "grok-4.6");
        assert_eq!(body["input"], "q");
        assert_eq!(body["store"], false);
        assert_eq!(body["temperature"], 0.1);
        assert_eq!(body["top_p"], 0.95);
        assert_eq!(body["max_output_tokens"], 8192);
        assert_eq!(body["tools"][0], json!({ "type": "web_search" }));
    }

    #[test]
    fn body_allowlist_only_has_no_excluded_key() {
        let mut request = request();
        request.allowed_domains = Some(vec!["docs.x.ai".into()]);
        let body = search_request_body(&request);
        let filters = &body["tools"][0]["filters"];
        assert_eq!(filters["allowed_domains"], json!(["docs.x.ai"]));
        assert!(filters.get("excluded_domains").is_none());
    }

    #[test]
    fn body_injects_excluded_domains() {
        let mut request = request();
        request.excluded_domains = Some(vec!["reddit.com".into()]);
        let body = search_request_body(&request);
        let filters = &body["tools"][0]["filters"];
        assert_eq!(filters["excluded_domains"], json!(["reddit.com"]));
        assert!(filters.get("allowed_domains").is_none());
    }

    #[test]
    fn empty_domain_lists_emit_bare_tool() {
        let mut request = request();
        request.allowed_domains = Some(Vec::new());
        request.excluded_domains = Some(Vec::new());
        let body = search_request_body(&request);
        assert_eq!(body["tools"][0], json!({ "type": "web_search" }));
    }

    #[test]
    fn x_body_matches_official_x_search_defaults() {
        let body = x_search_request_body(&x_request());
        assert_eq!(body["model"], "grok-4.6");
        assert_eq!(body["input"], "q");
        assert_eq!(body["store"], false);
        assert_eq!(body["temperature"], 0.1);
        assert_eq!(body["top_p"], 0.95);
        assert_eq!(body["max_output_tokens"], 8192);
        assert_eq!(body["tools"][0], json!({ "type": "x_search" }));
    }

    #[test]
    fn x_body_dates_match_grok_build_tool_entry() {
        let mut request = x_request();
        request.from_date = Some("2024-01-01".into());
        request.to_date = Some("2024-03-15".into());
        let body = x_search_request_body(&request);
        assert_eq!(
            body["tools"][0],
            json!({
                "type": "x_search",
                "from_date": "2024-01-01",
                "to_date": "2024-03-15",
            })
        );
    }

    #[test]
    fn x_body_empty_dates_and_handles_emit_bare_tool() {
        let mut request = x_request();
        request.from_date = Some(String::new());
        request.to_date = Some(String::new());
        request.allowed_x_handles = Some(Vec::new());
        request.excluded_x_handles = Some(Vec::new());
        let body = x_search_request_body(&request);
        assert_eq!(body["tools"][0], json!({ "type": "x_search" }));
    }

    #[test]
    fn x_body_forwards_official_handle_and_media_fields() {
        let mut request = x_request();
        request.allowed_x_handles = Some(vec!["elonmusk".into(), "xai".into()]);
        request.enable_image_understanding = true;
        request.enable_video_understanding = true;
        let body = x_search_request_body(&request);
        assert_eq!(
            body["tools"][0],
            json!({
                "type": "x_search",
                "allowed_x_handles": ["elonmusk", "xai"],
                "enable_image_understanding": true,
                "enable_video_understanding": true,
            })
        );
        assert!(body["tools"][0].get("excluded_x_handles").is_none());
    }

    #[test]
    fn x_body_excluded_handles_only() {
        let mut request = x_request();
        request.excluded_x_handles = Some(vec!["spam".into()]);
        let body = x_search_request_body(&request);
        assert_eq!(
            body["tools"][0],
            json!({
                "type": "x_search",
                "excluded_x_handles": ["spam"],
            })
        );
        assert!(body["tools"][0].get("allowed_x_handles").is_none());
    }

    fn response_json() -> Value {
        json!({
            "id": "resp_test",
            "object": "response",
            "created_at": 1234567890,
            "status": "completed",
            "model": "test-model",
            "output": [
                {
                    "type": "message",
                    "id": "msg_1",
                    "status": "completed",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Here is some info about Rust.",
                            "annotations": [
                                {
                                    "type": "url_citation",
                                    "url": "https://www.rust-lang.org/",
                                    "title": "Rust Programming Language",
                                    "start_index": 0,
                                    "end_index": 10
                                },
                                {
                                    "type": "url_citation",
                                    "url": "https://docs.rs/",
                                    "title": "Docs.rs",
                                    "start_index": 11,
                                    "end_index": 20
                                },
                                {
                                    "type": "url_citation",
                                    "url": "https://www.rust-lang.org/",
                                    "title": "Rust again",
                                    "start_index": 21,
                                    "end_index": 30
                                }
                            ]
                        }
                    ]
                }
            ]
        })
    }

    #[test]
    fn parse_extracts_text_and_deduped_citations() {
        let result = parse_search_response(&request(), response_json());
        assert_eq!(result.content, "Here is some info about Rust.");
        assert_eq!(
            result.citations,
            ["https://www.rust-lang.org/", "https://docs.rs/"]
        );
    }

    #[test]
    fn parse_x_extracts_text_and_echoes_bounds() {
        let mut request = x_request();
        request.from_date = Some("2024-01-01".into());
        let result = parse_x_search_response(&request, response_json());
        assert_eq!(result.content, "Here is some info about Rust.");
        assert_eq!(
            result.citations,
            ["https://www.rust-lang.org/", "https://docs.rs/"]
        );
        assert_eq!(result.from_date.as_deref(), Some("2024-01-01"));
        assert!(result.to_date.is_none());
    }

    #[test]
    fn parse_empty_output_uses_official_fallback() {
        let result = parse_search_response(
            &request(),
            json!({
                "id": "resp_test",
                "object": "response",
                "status": "completed",
                "output": []
            }),
        );
        assert_eq!(result.content, "No search results found.");
        assert!(result.citations.is_empty());
    }
}
