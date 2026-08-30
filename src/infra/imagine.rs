use std::path::Path;
use std::time::Duration;

use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, RequestBuilder, StatusCode};
use serde_json::{Value, json};
use xai_grok_auth::{HttpAuth, StaticAuthCredentialProvider};

use crate::error::{Error, Result};
use crate::model::auth::Credential;
use crate::model::media::{
    GeneratedImage, ImageGenerateRequest, ImageGenerateResult, VideoGenerateRequest,
    VideoGenerateResult, VideoTask,
};

const SESSION_TOKEN_HEADER: &str = "X-XAI-Token-Auth";
const SESSION_TOKEN_VALUE: &str = "xai-grok-cli";

pub const IMAGE_GEN_TOOL_NAME: &str = "image_gen";
pub const IMAGINE_COMMAND_NAME: &str = "imagine";
pub const IMAGE_TO_VIDEO_TOOL_NAME: &str = "image_to_video";
pub const IMAGINE_VIDEO_COMMAND_NAME: &str = "imagine-video";

pub fn official_user_agent() -> String {
    format!("grok-api/{}", env!("CARGO_PKG_VERSION"))
}

struct BearerAuth {
    token: String,
    session: bool,
}

impl HttpAuth for BearerAuth {
    fn apply(&self, builder: RequestBuilder, _base_url: &str) -> RequestBuilder {
        let builder = builder.bearer_auth(&self.token);
        if self.session {
            builder.header(SESSION_TOKEN_HEADER, SESSION_TOKEN_VALUE)
        } else {
            builder
        }
    }
}

pub struct ImagineClient {
    http: Client,
    base_url: String,
    auth: StaticAuthCredentialProvider,
}

impl ImagineClient {
    pub fn new(base_url: &str, credential: Credential, timeout: Duration) -> Result<Self> {
        let http = Client::builder()
            .timeout(timeout)
            .user_agent(official_user_agent())
            .build()?;
        let auth = StaticAuthCredentialProvider::new(
            Box::new(BearerAuth {
                token: credential.token.clone(),
                session: credential.is_session(),
            }),
            Some(credential.token.clone()),
        );
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            auth,
        })
    }

    fn headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    pub async fn generate_image(&self, request: &ImageGenerateRequest) -> Result<ImageGenerateResult> {
        let endpoint = if request.image.is_some() {
            format!("{}/images/edits", self.base_url)
        } else {
            format!("{}/images/generations", self.base_url)
        };
        let mut body = json!({
            "model": request.model,
            "prompt": request.prompt,
            "n": request.n,
            "response_format": request.response_format.as_api_value(),
        });
        if let Some(ratio) = &request.aspect_ratio {
            body["aspect_ratio"] = json!(ratio);
        }
        if let Some(resolution) = &request.resolution {
            body["resolution"] = json!(resolution);
        }
        if let Some(quality) = &request.quality {
            body["quality"] = json!(quality);
        }
        if let Some(image) = &request.image {
            body["image"] = json!({
                "url": image_ref(image)?,
                "type": "image_url",
            });
        }

        let value = self.post_json(&endpoint, body).await?;
        let model = value
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or(&request.model)
            .to_string();
        let images = parse_images(&value)?;
        Ok(ImageGenerateResult { model, images })
    }

    pub async fn start_video(&self, request: &VideoGenerateRequest) -> Result<VideoTask> {
        let endpoint = format!("{}/videos/generations", self.base_url);
        let mut body = json!({
            "model": request.model,
            "prompt": request.prompt,
        });
        if let Some(duration) = request.duration {
            body["duration"] = json!(duration);
        }
        if let Some(ratio) = &request.aspect_ratio {
            body["aspect_ratio"] = json!(ratio);
        }
        if let Some(image) = &request.image {
            body["image"] = json!({ "url": image_ref(image)? });
        }
        if !request.reference_images.is_empty() {
            let images: Result<Vec<Value>> = request
                .reference_images
                .iter()
                .map(|image| Ok(json!({ "url": image_ref(image)? })))
                .collect();
            body["reference_images"] = json!(images?);
        }
        let value = self.post_json(&endpoint, body).await?;
        parse_video_task(value)
    }

    pub async fn get_video(&self, request_id: &str) -> Result<VideoTask> {
        let endpoint = format!("{}/videos/{request_id}", self.base_url);
        let value = self.get_json(&endpoint).await?;
        parse_video_task(value)
    }

    pub async fn wait_video(&self, request_id: &str, poll_interval: Duration) -> Result<VideoTask> {
        loop {
            let task = self.get_video(request_id).await?;
            match task.status.as_str() {
                "done" | "completed" | "succeeded" | "success" => return Ok(task),
                "failed" | "expired" | "error" => {
                    return Err(Error::message(format!(
                        "video task {request_id} {}",
                        task.status
                    )));
                }
                _ => tokio::time::sleep(poll_interval).await,
            }
        }
    }

    pub async fn download(&self, url: &str, dest: &Path) -> Result<()> {
        let response = self.http.get(url).send().await?;
        if !response.status().is_success() {
            return Err(Error::Http {
                status: response.status().as_u16(),
                body: response.text().await.unwrap_or_default(),
            });
        }
        let bytes = response.bytes().await?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
        }
        std::fs::write(dest, bytes).map_err(|source| Error::io(dest, source))?;
        Ok(())
    }

    async fn post_json(&self, url: &str, body: Value) -> Result<Value> {
        let request = self.http.post(url).headers(self.headers()?).json(&body);
        let response = self.auth.apply(request, &self.base_url).send().await?;
        self.read_json(response).await
    }

    async fn get_json(&self, url: &str) -> Result<Value> {
        let request = self.http.get(url).headers(self.headers()?);
        let response = self.auth.apply(request, &self.base_url).send().await?;
        self.read_json(response).await
    }

    async fn read_json(&self, response: reqwest::Response) -> Result<Value> {
        let status = response.status();
        let text = response.text().await?;
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(Error::auth(format!("{status}: {text}")));
        }
        if !status.is_success() {
            return Err(Error::Http {
                status: status.as_u16(),
                body: text,
            });
        }
        Ok(serde_json::from_str(&text)?)
    }
}

fn parse_images(value: &Value) -> Result<Vec<GeneratedImage>> {
    let data = value
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| {
            value.get("url").map(|url| {
                vec![json!({
                    "url": url,
                    "b64_json": value.get("b64_json"),
                })]
            })
        })
        .unwrap_or_default();
    Ok(data
        .into_iter()
        .map(|item| GeneratedImage {
            url: item.get("url").and_then(Value::as_str).map(str::to_string),
            b64_json: item
                .get("b64_json")
                .and_then(Value::as_str)
                .map(str::to_string),
            path: None,
        })
        .collect())
}

fn parse_video_task(value: Value) -> Result<VideoTask> {
    if let Ok(task) = serde_json::from_value::<VideoTask>(value.clone()) {
        return Ok(task);
    }
    let request_id = value
        .get("request_id")
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
        .ok_or_else(|| Error::message("video response missing request_id"))?
        .to_string();
    Ok(VideoTask {
        request_id,
        status: value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("pending")
            .to_string(),
        video: value
            .get("video")
            .cloned()
            .and_then(|video| serde_json::from_value(video).ok()),
        error: value.get("error").cloned(),
    })
}

pub fn image_ref(input: &str) -> Result<String> {
    if input.starts_with("http://") || input.starts_with("https://") || input.starts_with("data:") {
        return Ok(input.to_string());
    }
    let path = Path::new(input);
    let bytes = std::fs::read(path).map_err(|source| Error::io(path, source))?;
    let mime = if input.ends_with(".png") {
        "image/png"
    } else if input.ends_with(".webp") {
        "image/webp"
    } else if input.ends_with(".gif") {
        "image/gif"
    } else {
        "image/jpeg"
    };
    use base64::Engine as _;
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

pub fn write_base64_image(b64: &str, dest: &Path) -> Result<()> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|err| Error::message(format!("invalid base64 image: {err}")))?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    }
    std::fs::write(dest, bytes).map_err(|source| Error::io(dest, source))?;
    Ok(())
}

pub fn video_result(task: VideoTask, path: Option<String>) -> VideoGenerateResult {
    VideoGenerateResult {
        request_id: task.request_id,
        status: task.status,
        url: task.video.and_then(|video| video.url),
        path,
    }
}
