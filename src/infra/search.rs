use serde_json::{Value, json};

use crate::error::Result;
use crate::infra::imagine::ImagineClient;
use crate::model::search::{WebSearchRequest, WebSearchResult};

/// Official `web_search` tool request fields from grok-build
/// `WebSearchClient::build_request_json`.
const STORE: bool = false;
const TEMPERATURE: f64 = 0.1;
const TOP_P: f64 = 0.95;
const MAX_OUTPUT_TOKENS: u32 = 8192;

pub async fn search(client: &ImagineClient, request: &WebSearchRequest) -> Result<WebSearchResult> {
    let endpoint = format!("{}/responses", client.base_url());
    let body = search_request_body(request);
    let value = client.post_json(&endpoint, body).await?;
    Ok(parse_search_response(request, value))
}

pub fn search_request_body(request: &WebSearchRequest) -> Value {
    json!({
        "model": request.model,
        "input": request.query,
        "tools": [web_search_tool_entry(
            request.allowed_domains.as_deref(),
            request.excluded_domains.as_deref(),
        )],
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

fn nonempty(domains: Option<&[String]>) -> Option<&[String]> {
    domains.filter(|domains| !domains.is_empty())
}

pub fn parse_search_response(request: &WebSearchRequest, value: Value) -> WebSearchResult {
    WebSearchResult {
        query: request.query.clone(),
        content: output_text(&value).unwrap_or_else(|| "No search results found.".to_string()),
        citations: extract_citations(&value),
        allowed_domains: request.allowed_domains.clone(),
    }
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
    use crate::model::search::WebSearchRequest;

    fn request() -> WebSearchRequest {
        WebSearchRequest {
            query: "q".into(),
            model: "grok-4.6".into(),
            allowed_domains: None,
            excluded_domains: None,
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
