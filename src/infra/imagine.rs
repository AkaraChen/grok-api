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

    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    fn headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    pub async fn generate_image(
        &self,
        request: &ImageGenerateRequest,
    ) -> Result<ImageGenerateResult> {
        let endpoint = if request.image.is_some() {
            format!("{}/images/edits", self.base_url)
        } else {
            format!("{}/images/generations", self.base_url)
        };
        let body = image_generate_body(request)?;
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
        let body = video_generate_body(request)?;
        let value = self.post_json(&endpoint, body).await?;
        parse_video_task(value)
    }

    pub async fn list_models(&self) -> Result<Value> {
        let endpoint = format!("{}/models", self.base_url);
        self.get_json(&endpoint).await
    }

    pub async fn list_voices(&self) -> Result<Value> {
        let endpoint = format!("{}/tts/voices", self.base_url);
        self.get_json(&endpoint).await
    }

    pub async fn get_video(&self, request_id: &str) -> Result<VideoTask> {
        let endpoint = format!("{}/videos/{request_id}", self.base_url);
        let value = self.get_json(&endpoint).await?;
        let mut task = parse_video_task(value)?;
        if task.request_id.is_empty() {
            task.request_id = request_id.to_string();
        }
        Ok(task)
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

    pub(crate) async fn post_json(&self, url: &str, body: Value) -> Result<Value> {
        let request = self.http.post(url).headers(self.headers()?).json(&body);
        let response = self
            .auth
            .apply(request, &self.base_url)
            .send()
            .await
            .map_err(|err| Error::message(format!("HTTP POST {url} failed: {err}")))?;
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
        .unwrap_or_default()
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

pub fn image_generate_body(request: &ImageGenerateRequest) -> Result<Value> {
    let mut body = json!({
        "model": request.model,
        "prompt": request.prompt,
        "n": request.n,
        "response_format": request.response_format.as_api_value(),
        "aspect_ratio": request.aspect_ratio.as_deref().unwrap_or("auto"),
        "resolution": request.resolution.as_deref().unwrap_or("1k"),
    });
    if let Some(quality) = &request.quality {
        body["quality"] = json!(quality);
    }
    if let Some(image) = &request.image {
        body["image"] = json!({
            "url": image_ref(image)?,
            "type": "image_url",
        });
    }
    Ok(body)
}

pub fn video_generate_body(request: &VideoGenerateRequest) -> Result<Value> {
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
    if let Some(resolution) = &request.resolution {
        body["resolution"] = json!(resolution);
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
    if !request.voices.is_empty() {
        body["reference_audios"] = json!(
            request
                .voices
                .iter()
                .map(|voice_id| json!({ "voice_id": voice_id }))
                .collect::<Vec<_>>()
        );
    }
    Ok(body)
}

pub fn filter_imagine_image_models(value: &Value) -> Vec<Value> {
    let data = value
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .cloned()
        .unwrap_or_default();
    data.into_iter()
        .filter(|model| {
            let id = model.get("id").and_then(Value::as_str).unwrap_or("");
            if id.starts_with("grok-imagine-video") {
                return false;
            }
            id.starts_with("grok-imagine-image") || model.get("image_price").is_some()
        })
        .collect()
}

pub fn voice_entries(value: &Value) -> Vec<Value> {
    value
        .get("voices")
        .or_else(|| value.get("data"))
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .cloned()
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::media::{ImageGenerateRequest, ResponseFormat, VideoGenerateRequest};

    fn image_request() -> ImageGenerateRequest {
        ImageGenerateRequest {
            prompt: "a cat".into(),
            model: "grok-imagine-image-2.0".into(),
            aspect_ratio: None,
            n: 1,
            resolution: None,
            quality: None,
            response_format: ResponseFormat::Url,
            image: None,
        }
    }

    fn video_request() -> VideoGenerateRequest {
        VideoGenerateRequest {
            prompt: "ocean waves".into(),
            model: "grok-imagine-video-1.5".into(),
            image: None,
            reference_images: Vec::new(),
            voices: Vec::new(),
            duration: None,
            aspect_ratio: None,
            resolution: None,
            wait: false,
            poll_interval_secs: 5,
        }
    }

    #[test]
    fn image_body_sends_defaults_and_wire_response_format() {
        let body = image_generate_body(&image_request()).unwrap();
        assert_eq!(body["aspect_ratio"], "auto");
        assert_eq!(body["resolution"], "1k");
        assert_eq!(body["response_format"], "url");
        assert!(body.get("quality").is_none());
    }

    #[test]
    fn image_body_maps_base64_to_b64_json() {
        let mut request = image_request();
        request.response_format = ResponseFormat::Base64;
        request.aspect_ratio = Some("16:9".into());
        request.resolution = Some("2k".into());
        request.quality = Some("auto".into());
        let body = image_generate_body(&request).unwrap();
        assert_eq!(body["response_format"], "b64_json");
        assert_eq!(body["aspect_ratio"], "16:9");
        assert_eq!(body["resolution"], "2k");
        assert_eq!(body["quality"], "auto");
    }

    #[test]
    fn video_body_omits_resolution_by_default() {
        let body = video_generate_body(&video_request()).unwrap();
        assert!(body.get("resolution").is_none());
        assert!(body.get("reference_audios").is_none());
    }

    #[test]
    fn video_body_sends_resolution_on_text_and_image_to_video() {
        let mut t2v = video_request();
        t2v.resolution = Some("720p".into());
        let t2v_body = video_generate_body(&t2v).unwrap();
        assert_eq!(t2v_body["resolution"], "720p");
        assert!(t2v_body.get("image").is_none());

        let mut i2v = video_request();
        i2v.image = Some("https://example.com/start.png".into());
        i2v.resolution = Some("1080p".into());
        let i2v_body = video_generate_body(&i2v).unwrap();
        assert_eq!(i2v_body["resolution"], "1080p");
        assert_eq!(i2v_body["image"]["url"], "https://example.com/start.png");
    }

    #[test]
    fn video_body_sends_reference_audios() {
        let mut request = video_request();
        request.voices = vec!["eve".into(), "leo".into()];
        let body = video_generate_body(&request).unwrap();
        assert_eq!(
            body["reference_audios"],
            json!([{ "voice_id": "eve" }, { "voice_id": "leo" }])
        );
    }

    #[test]
    fn filters_imagine_image_models() {
        let payload = json!({
            "data": [
                { "id": "grok-imagine-image-2.0", "owned_by": "xai" },
                { "id": "grok-imagine-image-quality" },
                { "id": "grok-imagine-video-1.5", "image_price": 1 },
                { "id": "grok-3", "image_price": 2 },
                { "id": "grok-3" }
            ]
        });
        let models = filter_imagine_image_models(&payload);
        let ids: Vec<_> = models
            .iter()
            .map(|model| model["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            ids,
            [
                "grok-imagine-image-2.0",
                "grok-imagine-image-quality",
                "grok-3"
            ]
        );
    }
}
