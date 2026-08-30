use serde::{Deserialize, Serialize};

use super::auth::AuthSource;

pub const DEFAULT_BASE_URL: &str = "https://api.x.ai/v1";
pub const DEFAULT_IMAGE_MODEL: &str = "grok-imagine-image-2.0";
pub const DEFAULT_VIDEO_MODEL: &str = "grok-imagine-video-1.5";
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

impl OutputFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub auth_source: AuthSource,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub output: OutputFormat,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_image_model")]
    pub default_image_model: String,
    #[serde(default = "default_video_model")]
    pub default_video_model: String,
}

fn default_base_url() -> String {
    DEFAULT_BASE_URL.to_string()
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

fn default_image_model() -> String {
    DEFAULT_IMAGE_MODEL.to_string()
}

fn default_video_model() -> String {
    DEFAULT_VIDEO_MODEL.to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            auth_source: AuthSource::default(),
            base_url: default_base_url(),
            output: OutputFormat::default(),
            timeout: default_timeout(),
            api_key: None,
            default_image_model: default_image_model(),
            default_video_model: default_video_model(),
        }
    }
}

impl AppConfig {
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), &'static str> {
        match key {
            "auth_source" => {
                self.auth_source = AuthSource::parse(value).ok_or("auth_source")?;
            }
            "base_url" => self.base_url = value.to_string(),
            "output" => {
                self.output = OutputFormat::parse(value).ok_or("output")?;
            }
            "timeout" => {
                self.timeout = value.parse().map_err(|_| "timeout")?;
            }
            "api_key" => self.api_key = Some(value.to_string()),
            "default_image_model" => self.default_image_model = value.to_string(),
            "default_video_model" => self.default_video_model = value.to_string(),
            _ => return Err("unknown key"),
        }
        Ok(())
    }
}
