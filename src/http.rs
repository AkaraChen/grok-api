use std::path::PathBuf;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_swagger_ui::SwaggerUi;

use crate::error::{Error, Result as ApiResult};
use crate::model::auth::AuthStatus;
use crate::model::config::{AppConfig, DEFAULT_POLL_INTERVAL_SECS};
use crate::model::media::{
    ImageGenerateRequest, ImageGenerateResult, ResponseFormat, VideoGenerateRequest,
    VideoGenerateResult, VideoTask, image_refs,
};
use crate::model::search::{WebSearchRequest, WebSearchResult, XSearchRequest, XSearchResult};
use crate::service::auth::{self, AuthContext};
use crate::service::{config as config_service, image, search, video};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "grok-api",
        description = "Thin HTTP glue over grok-api CLI resources. Same fields as the CLI / official Imagine, web_search, and x_search payloads."
    ),
    tags(
        (name = "auth", description = "Authentication status"),
        (name = "config", description = "CLI configuration"),
        (name = "image", description = "Imagine image generation"),
        (name = "video", description = "Imagine video generation"),
        (name = "search", description = "Official web_search and x_search via POST /responses")
    )
)]
struct ApiDoc;

#[derive(Serialize, ToSchema)]
struct ErrorBody {
    error: String,
}

#[derive(Serialize, ToSchema)]
struct ConfigShowResponse {
    config: AppConfig,
    environment: String,
}

#[derive(Debug, Deserialize, ToSchema)]
struct ImageGenerateBody {
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

#[derive(Debug, Deserialize, ToSchema)]
struct VideoGenerateBody {
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

#[derive(Debug, Deserialize, ToSchema)]
struct VideoDownloadBody {
    file_id: String,
    out: String,
}

#[derive(Debug, Deserialize, ToSchema)]
struct SearchQueryBody {
    query: String,
    allowed_domains: Option<Vec<String>>,
    excluded_domains: Option<Vec<String>>,
    model: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
struct SearchXBody {
    query: String,
    from_date: Option<String>,
    to_date: Option<String>,
    allowed_x_handles: Option<Vec<String>>,
    excluded_x_handles: Option<Vec<String>>,
    enable_image_understanding: Option<bool>,
    enable_video_understanding: Option<bool>,
    model: Option<String>,
}

struct ApiError(Error);

impl From<Error> for ApiError {
    fn from(error: Error) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self.0 {
            Error::Http { status, body } => {
                let status = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
                match serde_json::from_str::<serde_json::Value>(&body) {
                    Ok(value) => (status, Json(value)).into_response(),
                    Err(_) => json_err(status, body),
                }
            }
            Error::Auth(message) => json_err(StatusCode::UNAUTHORIZED, message),
            Error::InvalidValue { flag, value } => json_err(
                StatusCode::BAD_REQUEST,
                format!("invalid value for {flag}: {value}"),
            ),
            Error::Path(error) => json_err(StatusCode::BAD_REQUEST, error.to_string()),
            error => json_err(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        }
    }
}

fn json_err(status: StatusCode, message: impl ToString) -> Response {
    (
        status,
        Json(ErrorBody {
            error: message.to_string(),
        }),
    )
        .into_response()
}

fn api_router() -> OpenApiRouter<AuthContext> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(auth_status))
        .routes(routes!(config_show))
        .routes(routes!(image_generate))
        .routes(routes!(image_model_list))
        .routes(routes!(video_generate))
        .routes(routes!(video_task_get))
        .routes(routes!(video_download))
        .routes(routes!(video_voice_list))
        .routes(routes!(search_query))
        .routes(routes!(search_x))
}

pub fn router(ctx: AuthContext) -> axum::Router {
    let (router, api) = api_router().split_for_parts();
    router
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", api))
        .with_state(ctx)
}

pub async fn serve(ctx: AuthContext, bind: &str) -> ApiResult<()> {
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|source| Error::io(bind, source))?;
    let addr = listener
        .local_addr()
        .map_err(|source| Error::io(bind, source))?;
    eprintln!("grok-api http listening on http://{addr}");
    eprintln!("openapi docs: http://{addr}/docs");
    axum::serve(listener, router(ctx))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|err| Error::message(err.to_string()))?;
    Ok(())
}

#[utoipa::path(
    get,
    path = "/auth/status",
    responses(
        (status = 200, description = "Current authentication state", body = AuthStatus),
        (status = 500, description = "Server error", body = ErrorBody)
    ),
    tag = "auth"
)]
async fn auth_status(State(ctx): State<AuthContext>) -> Result<Json<AuthStatus>, ApiError> {
    Ok(Json(auth::status(&ctx)?))
}

#[utoipa::path(
    get,
    path = "/config",
    responses(
        (status = 200, description = "Current ~/.grok-api configuration", body = ConfigShowResponse),
        (status = 500, description = "Server error", body = ErrorBody)
    ),
    tag = "config"
)]
async fn config_show() -> Result<Json<ConfigShowResponse>, ApiError> {
    Ok(Json(ConfigShowResponse {
        config: config_service::load()?,
        environment: xai_grok_env::GrokBuildEnvironment::Production.to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/image/generate",
    request_body = ImageGenerateBody,
    responses(
        (status = 200, description = "Generated images", body = ImageGenerateResult),
        (status = 400, description = "Invalid request", body = ErrorBody),
        (status = 401, description = "Not authenticated", body = ErrorBody)
    ),
    tag = "image"
)]
async fn image_generate(
    State(ctx): State<AuthContext>,
    Json(body): Json<ImageGenerateBody>,
) -> Result<Json<ImageGenerateResult>, ApiError> {
    let response_format = body.response_format.as_deref().unwrap_or("url");
    let response_format =
        ResponseFormat::parse(response_format).ok_or_else(|| Error::InvalidValue {
            flag: "response-format",
            value: response_format.to_string(),
        })?;
    let result = image::generate(
        &ctx,
        image::ImageGenerateOpts {
            request: ImageGenerateRequest {
                prompt: body.prompt,
                model: body
                    .model
                    .unwrap_or_else(|| ctx.config.default_image_model.clone()),
                aspect_ratio: body.aspect_ratio,
                n: body.n.unwrap_or(1),
                resolution: body.resolution,
                quality: body.quality,
                response_format,
                images: image_refs(body.image, body.images),
            },
            out: body.out.map(PathBuf::from),
            out_dir: body.out_dir.map(PathBuf::from),
            out_prefix: body.out_prefix.unwrap_or_else(|| "image".into()),
            dry_run: false,
        },
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/image/models",
    responses(
        (status = 200, description = "Imagine image models from GET /models", body = Vec<Object>),
        (status = 401, description = "Not authenticated", body = ErrorBody)
    ),
    tag = "image"
)]
async fn image_model_list(
    State(ctx): State<AuthContext>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    Ok(Json(image::list_models(&ctx).await?))
}

#[utoipa::path(
    post,
    path = "/video/generate",
    request_body = VideoGenerateBody,
    responses(
        (status = 200, description = "Video task or completed video", body = VideoGenerateResult),
        (status = 400, description = "Invalid request", body = ErrorBody),
        (status = 401, description = "Not authenticated", body = ErrorBody)
    ),
    tag = "video"
)]
async fn video_generate(
    State(ctx): State<AuthContext>,
    Json(body): Json<VideoGenerateBody>,
) -> Result<Json<VideoGenerateResult>, ApiError> {
    let result = video::generate(
        &ctx,
        VideoGenerateRequest {
            prompt: body.prompt,
            model: body
                .model
                .unwrap_or_else(|| ctx.config.default_video_model.clone()),
            image: body.image,
            reference_images: body.reference_images.unwrap_or_default(),
            voices: body.voices.unwrap_or_default(),
            duration: body.duration,
            aspect_ratio: body.aspect_ratio,
            resolution: body.resolution,
            wait: body.wait.unwrap_or(true),
            poll_interval_secs: body.poll_interval.unwrap_or(DEFAULT_POLL_INTERVAL_SECS),
        },
        body.download.map(PathBuf::from),
        false,
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/video/tasks/{task_id}",
    params(("task_id" = String, Path, description = "Video generation task ID")),
    responses(
        (status = 200, description = "Video task status", body = VideoTask),
        (status = 401, description = "Not authenticated", body = ErrorBody)
    ),
    tag = "video"
)]
async fn video_task_get(
    State(ctx): State<AuthContext>,
    Path(task_id): Path<String>,
) -> Result<Json<VideoTask>, ApiError> {
    Ok(Json(video::task_get(&ctx, &task_id).await?))
}

#[utoipa::path(
    post,
    path = "/video/download",
    request_body = VideoDownloadBody,
    responses(
        (status = 200, description = "Downloaded video", body = VideoGenerateResult),
        (status = 400, description = "Invalid request", body = ErrorBody),
        (status = 401, description = "Not authenticated", body = ErrorBody)
    ),
    tag = "video"
)]
async fn video_download(
    State(ctx): State<AuthContext>,
    Json(body): Json<VideoDownloadBody>,
) -> Result<Json<VideoGenerateResult>, ApiError> {
    Ok(Json(
        video::download(&ctx, &body.file_id, &PathBuf::from(body.out)).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/video/voices",
    responses(
        (status = 200, description = "TTS voices from GET /tts/voices", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody)
    ),
    tag = "video"
)]
async fn video_voice_list(
    State(ctx): State<AuthContext>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(video::list_voices(&ctx).await?))
}

#[utoipa::path(
    post,
    path = "/search/query",
    request_body = SearchQueryBody,
    responses(
        (status = 200, description = "web_search result", body = WebSearchResult),
        (status = 400, description = "Invalid request", body = ErrorBody),
        (status = 401, description = "Not authenticated", body = ErrorBody)
    ),
    tag = "search"
)]
async fn search_query(
    State(ctx): State<AuthContext>,
    Json(body): Json<SearchQueryBody>,
) -> Result<Json<WebSearchResult>, ApiError> {
    Ok(Json(
        search::query(
            &ctx,
            WebSearchRequest {
                query: body.query,
                model: body
                    .model
                    .unwrap_or_else(|| ctx.config.default_search_model.clone()),
                allowed_domains: nonempty(body.allowed_domains),
                excluded_domains: nonempty(body.excluded_domains),
            },
            false,
        )
        .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/search/x",
    request_body = SearchXBody,
    responses(
        (status = 200, description = "x_search result", body = XSearchResult),
        (status = 400, description = "Invalid request", body = ErrorBody),
        (status = 401, description = "Not authenticated", body = ErrorBody)
    ),
    tag = "search"
)]
async fn search_x(
    State(ctx): State<AuthContext>,
    Json(body): Json<SearchXBody>,
) -> Result<Json<XSearchResult>, ApiError> {
    Ok(Json(
        search::x_query(
            &ctx,
            XSearchRequest {
                query: body.query,
                model: body
                    .model
                    .unwrap_or_else(|| ctx.config.default_search_model.clone()),
                from_date: nonempty_string(body.from_date),
                to_date: nonempty_string(body.to_date),
                allowed_x_handles: nonempty(body.allowed_x_handles),
                excluded_x_handles: nonempty(body.excluded_x_handles),
                enable_image_understanding: body.enable_image_understanding.unwrap_or(false),
                enable_video_understanding: body.enable_video_understanding.unwrap_or(false),
            },
            false,
        )
        .await?,
    ))
}

fn nonempty(values: Option<Vec<String>>) -> Option<Vec<String>> {
    values.filter(|values| !values.is_empty())
}

fn nonempty_string(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use utoipa::OpenApi;

    use super::*;

    #[test]
    fn openapi_lists_cli_resource_paths() {
        let (_, api) = api_router().split_for_parts();
        let json = api.to_json().expect("openapi json");
        for path in [
            "/auth/status",
            "/config",
            "/image/generate",
            "/image/models",
            "/video/generate",
            "/video/tasks/{task_id}",
            "/video/download",
            "/video/voices",
            "/search/query",
            "/search/x",
        ] {
            assert!(json.contains(&format!("\"{path}\"")), "missing {path}");
        }
        assert!(json.contains("grok-api"));
        let _ = ApiDoc::openapi();
    }
}
