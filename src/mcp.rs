use std::path::PathBuf;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt, schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result as ApiResult};
use crate::model::config::DEFAULT_POLL_INTERVAL_SECS;
use crate::model::media::{ImageGenerateRequest, ResponseFormat, VideoGenerateRequest, image_refs};
use crate::model::search::{WebSearchRequest, XSearchRequest};
use crate::service::auth::{self, AuthContext};
use crate::service::{config as config_service, image, search, video};

#[derive(Clone)]
pub struct GrokApiMcp {
    ctx: AuthContext,
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

impl GrokApiMcp {
    pub fn new(ctx: AuthContext) -> Self {
        Self {
            ctx,
            tool_router: Self::tool_router(),
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct ImageGenerateArgs {
    /// Image description
    prompt: String,
    aspect_ratio: Option<String>,
    n: Option<u32>,
    out: Option<String>,
    out_dir: Option<String>,
    out_prefix: Option<String>,
    model: Option<String>,
    resolution: Option<String>,
    quality: Option<String>,
    response_format: Option<String>,
    image: Option<String>,
    images: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct VideoGenerateArgs {
    /// Video description
    prompt: String,
    model: Option<String>,
    image: Option<String>,
    reference_images: Option<Vec<String>>,
    duration: Option<u32>,
    aspect_ratio: Option<String>,
    resolution: Option<String>,
    voices: Option<Vec<String>>,
    download: Option<String>,
    wait: Option<bool>,
    poll_interval: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct VideoTaskGetArgs {
    task_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct VideoDownloadArgs {
    file_id: String,
    out: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SearchQueryArgs {
    query: String,
    allowed_domains: Option<Vec<String>>,
    excluded_domains: Option<Vec<String>>,
    model: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SearchXArgs {
    query: String,
    from_date: Option<String>,
    to_date: Option<String>,
    allowed_x_handles: Option<Vec<String>>,
    excluded_x_handles: Option<Vec<String>>,
    enable_image_understanding: Option<bool>,
    enable_video_understanding: Option<bool>,
    model: Option<String>,
}

#[tool_router]
impl GrokApiMcp {
    #[tool(description = "Generate or edit an image via grok-api image generate")]
    async fn image_generate(
        &self,
        Parameters(args): Parameters<ImageGenerateArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let response_format = args.response_format.as_deref().unwrap_or("url");
        let response_format = ResponseFormat::parse(response_format).ok_or_else(|| {
            McpError::invalid_params(format!("invalid response_format: {response_format}"), None)
        })?;
        match image::generate(
            &self.ctx,
            image::ImageGenerateOpts {
                request: ImageGenerateRequest {
                    prompt: args.prompt,
                    model: args
                        .model
                        .unwrap_or_else(|| self.ctx.config.default_image_model.clone()),
                    aspect_ratio: args.aspect_ratio,
                    n: args.n.unwrap_or(1),
                    resolution: args.resolution,
                    quality: args.quality,
                    response_format,
                    images: image_refs(args.image, args.images),
                },
                out: args.out.map(PathBuf::from),
                out_dir: args.out_dir.map(PathBuf::from),
                out_prefix: args.out_prefix.unwrap_or_else(|| "image".into()),
                dry_run: false,
            },
        )
        .await
        {
            Ok(result) => json_ok(&result),
            Err(error) => Ok(tool_err(error)),
        }
    }

    #[tool(description = "List Imagine image models via grok-api image model list")]
    async fn image_model_list(&self) -> std::result::Result<CallToolResult, McpError> {
        match image::list_models(&self.ctx).await {
            Ok(models) => json_ok(&models),
            Err(error) => Ok(tool_err(error)),
        }
    }

    #[tool(description = "Generate a video via grok-api video generate")]
    async fn video_generate(
        &self,
        Parameters(args): Parameters<VideoGenerateArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        match video::generate(
            &self.ctx,
            VideoGenerateRequest {
                prompt: args.prompt,
                model: args
                    .model
                    .unwrap_or_else(|| self.ctx.config.default_video_model.clone()),
                image: args.image,
                reference_images: args.reference_images.unwrap_or_default(),
                voices: args.voices.unwrap_or_default(),
                duration: args.duration,
                aspect_ratio: args.aspect_ratio,
                resolution: args.resolution,
                wait: args.wait.unwrap_or(true),
                poll_interval_secs: args.poll_interval.unwrap_or(DEFAULT_POLL_INTERVAL_SECS),
            },
            args.download.map(PathBuf::from),
            false,
        )
        .await
        {
            Ok(result) => json_ok(&result),
            Err(error) => Ok(tool_err(error)),
        }
    }

    #[tool(description = "Query a video task via grok-api video task get")]
    async fn video_task_get(
        &self,
        Parameters(args): Parameters<VideoTaskGetArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        match video::task_get(&self.ctx, &args.task_id).await {
            Ok(task) => json_ok(&task),
            Err(error) => Ok(tool_err(error)),
        }
    }

    #[tool(description = "Download a completed video via grok-api video download")]
    async fn video_download(
        &self,
        Parameters(args): Parameters<VideoDownloadArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        match video::download(&self.ctx, &args.file_id, &PathBuf::from(args.out)).await {
            Ok(result) => json_ok(&result),
            Err(error) => Ok(tool_err(error)),
        }
    }

    #[tool(description = "List TTS voices via grok-api video voice list")]
    async fn video_voice_list(&self) -> std::result::Result<CallToolResult, McpError> {
        match video::list_voices(&self.ctx).await {
            Ok(voices) => json_ok(&voices),
            Err(error) => Ok(tool_err(error)),
        }
    }

    #[tool(description = "Search the web via grok-api search query (official web_search)")]
    async fn search_query(
        &self,
        Parameters(args): Parameters<SearchQueryArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        match search::query(
            &self.ctx,
            WebSearchRequest {
                query: args.query,
                model: args
                    .model
                    .unwrap_or_else(|| self.ctx.config.default_search_model.clone()),
                allowed_domains: nonempty(args.allowed_domains),
                excluded_domains: nonempty(args.excluded_domains),
            },
            false,
        )
        .await
        {
            Ok(result) => json_ok(&result),
            Err(error) => Ok(tool_err(error)),
        }
    }

    #[tool(description = "Search X via grok-api search x (official x_search)")]
    async fn search_x(
        &self,
        Parameters(args): Parameters<SearchXArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        match search::x_query(
            &self.ctx,
            XSearchRequest {
                query: args.query,
                model: args
                    .model
                    .unwrap_or_else(|| self.ctx.config.default_search_model.clone()),
                from_date: nonempty_string(args.from_date),
                to_date: nonempty_string(args.to_date),
                allowed_x_handles: nonempty(args.allowed_x_handles),
                excluded_x_handles: nonempty(args.excluded_x_handles),
                enable_image_understanding: args.enable_image_understanding.unwrap_or(false),
                enable_video_understanding: args.enable_video_understanding.unwrap_or(false),
            },
            false,
        )
        .await
        {
            Ok(result) => json_ok(&result),
            Err(error) => Ok(tool_err(error)),
        }
    }

    #[tool(description = "Show grok-api authentication status")]
    fn auth_status(&self) -> std::result::Result<CallToolResult, McpError> {
        match auth::status(&self.ctx) {
            Ok(status) => json_ok(&status),
            Err(error) => Ok(tool_err(error)),
        }
    }

    #[tool(description = "Show grok-api configuration")]
    fn config_show(&self) -> std::result::Result<CallToolResult, McpError> {
        match config_service::load() {
            Ok(config) => json_ok(&serde_json::json!({
                "config": config,
                "environment": xai_grok_env::GrokBuildEnvironment::Production.to_string(),
            })),
            Err(error) => Ok(tool_err(error)),
        }
    }
}

#[tool_handler(
    name = "grok-api",
    version = "0.1.0",
    instructions = "Thin grok-api MCP: image generate, video generate/task/download, official web_search and x_search, auth status. Do not invent Imagine or search fields.",
    router = self.tool_router
)]
impl ServerHandler for GrokApiMcp {}

pub async fn serve_stdio(ctx: AuthContext) -> ApiResult<()> {
    let server = GrokApiMcp::new(ctx)
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|err| Error::message(err.to_string()))?;
    server
        .waiting()
        .await
        .map_err(|err| Error::message(err.to_string()))?;
    Ok(())
}

pub async fn serve_http(ctx: AuthContext, bind: &str) -> ApiResult<()> {
    let service = StreamableHttpService::new(
        {
            let ctx = ctx.clone();
            move || Ok(GrokApiMcp::new(ctx.clone()))
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );
    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|source| Error::io(bind, source))?;
    let addr = listener
        .local_addr()
        .map_err(|source| Error::io(bind, source))?;
    eprintln!("grok-api mcp listening on http://{addr}/mcp");
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|err| Error::message(err.to_string()))?;
    Ok(())
}

fn nonempty(values: Option<Vec<String>>) -> Option<Vec<String>> {
    values.filter(|values| !values.is_empty())
}

fn nonempty_string(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}

fn json_ok<T: Serialize>(value: &T) -> std::result::Result<CallToolResult, McpError> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| McpError::internal_error(err.to_string(), None))?;
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

fn tool_err(error: Error) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(error.to_string())])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_router_exposes_cli_resources() {
        let router = GrokApiMcp::tool_router();
        for name in [
            "image_generate",
            "image_model_list",
            "video_generate",
            "video_task_get",
            "video_download",
            "video_voice_list",
            "search_query",
            "search_x",
            "auth_status",
            "config_show",
        ] {
            assert!(router.has_route(name), "missing tool {name}");
        }
    }

    fn parse<T: for<'de> Deserialize<'de>>(value: serde_json::Value) -> T {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn image_args_forward_official_fields() {
        let args: ImageGenerateArgs = parse(serde_json::json!({
            "prompt": "a cat",
            "aspect_ratio": "16:9",
            "n": 1,
            "quality": "auto"
        }));
        assert_eq!(args.prompt, "a cat");
        assert_eq!(args.aspect_ratio.as_deref(), Some("16:9"));
        assert_eq!(args.n, Some(1));
        assert_eq!(args.quality.as_deref(), Some("auto"));
    }

    #[test]
    fn image_args_merge_single_and_multi_refs() {
        assert_eq!(
            image_refs(Some("a.png".into()), None),
            vec!["a.png".to_string()]
        );
        assert_eq!(
            image_refs(
                Some("ignored.png".into()),
                Some(vec!["a.png".into(), "b.png".into()])
            ),
            vec!["a.png".to_string(), "b.png".to_string()]
        );
    }

    #[test]
    fn search_args_forward_official_domain_filters() {
        let args: SearchQueryArgs = parse(serde_json::json!({
            "query": "tokio",
            "allowed_domains": ["docs.rs"]
        }));
        assert_eq!(args.query, "tokio");
        assert_eq!(args.allowed_domains, Some(vec!["docs.rs".into()]));
        assert!(args.excluded_domains.is_none());
    }

    #[test]
    fn search_x_args_forward_official_x_search_fields() {
        let args: SearchXArgs = parse(serde_json::json!({
            "query": "xAI on X",
            "from_date": "2024-01-01",
            "allowed_x_handles": ["elonmusk"],
            "enable_image_understanding": true
        }));
        assert_eq!(args.query, "xAI on X");
        assert_eq!(args.from_date.as_deref(), Some("2024-01-01"));
        assert!(args.to_date.is_none());
        assert_eq!(args.allowed_x_handles, Some(vec!["elonmusk".into()]));
        assert!(args.excluded_x_handles.is_none());
        assert_eq!(args.enable_image_understanding, Some(true));
        assert!(args.enable_video_understanding.is_none());
    }
}
