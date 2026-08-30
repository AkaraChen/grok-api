use serde::Serialize;

#[derive(Debug, Clone)]
pub struct WebSearchRequest {
    pub query: String,
    pub model: String,
    pub allowed_domains: Option<Vec<String>>,
    pub excluded_domains: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WebSearchResult {
    pub query: String,
    pub content: String,
    pub citations: Vec<String>,
    pub allowed_domains: Option<Vec<String>>,
}

/// Official `x_search` tool fields from grok-build `XSearchOptions::to_tool_entry`
/// (`from_date` / `to_date`) plus the public Responses `x_search` handle and
/// media-understanding keys documented at docs.x.ai.
#[derive(Debug, Clone)]
pub struct XSearchRequest {
    pub query: String,
    pub model: String,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub allowed_x_handles: Option<Vec<String>>,
    pub excluded_x_handles: Option<Vec<String>>,
    pub enable_image_understanding: bool,
    pub enable_video_understanding: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct XSearchResult {
    pub query: String,
    pub content: String,
    pub citations: Vec<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub allowed_x_handles: Option<Vec<String>>,
    pub excluded_x_handles: Option<Vec<String>>,
}
