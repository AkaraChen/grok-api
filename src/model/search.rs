use serde::Serialize;

#[derive(Debug, Clone)]
pub struct WebSearchRequest {
    pub query: String,
    pub model: String,
    pub allowed_domains: Option<Vec<String>>,
    pub excluded_domains: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebSearchResult {
    pub query: String,
    pub content: String,
    pub citations: Vec<String>,
    pub allowed_domains: Option<Vec<String>>,
}
