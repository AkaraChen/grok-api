use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseFormat {
    #[default]
    Url,
    Base64,
}

impl ResponseFormat {
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Url => "url",
            Self::Base64 => "b64_json",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "url" => Some(Self::Url),
            "base64" | "b64_json" => Some(Self::Base64),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImageGenerateRequest {
    pub prompt: String,
    pub model: String,
    pub aspect_ratio: Option<String>,
    pub n: u32,
    pub resolution: Option<String>,
    pub quality: Option<String>,
    pub response_format: ResponseFormat,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct GeneratedImage {
    pub url: Option<String>,
    pub b64_json: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ImageGenerateResult {
    pub model: String,
    pub images: Vec<GeneratedImage>,
}

#[derive(Debug, Clone)]
pub struct VideoGenerateRequest {
    pub prompt: String,
    pub model: String,
    pub image: Option<String>,
    pub reference_images: Vec<String>,
    pub voices: Vec<String>,
    pub duration: Option<u32>,
    pub aspect_ratio: Option<String>,
    pub resolution: Option<String>,
    pub wait: bool,
    pub poll_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VideoTask {
    pub request_id: String,
    pub status: String,
    #[serde(default)]
    pub video: Option<VideoAsset>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VideoAsset {
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct VideoGenerateResult {
    pub request_id: String,
    pub status: String,
    pub url: Option<String>,
    pub path: Option<String>,
}
