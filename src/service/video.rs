use std::path::PathBuf;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::infra::imagine::{ImagineClient, video_result};
use crate::infra::paths::abs;
use crate::model::media::{VideoGenerateRequest, VideoGenerateResult, VideoTask};
use crate::service::auth::{self, AuthContext};

pub async fn generate(
    ctx: &AuthContext,
    request: VideoGenerateRequest,
    download: Option<PathBuf>,
    dry_run: bool,
) -> Result<VideoGenerateResult> {
    if dry_run {
        return Ok(VideoGenerateResult {
            request_id: "dry-run".to_string(),
            status: "dry-run".to_string(),
            url: None,
            path: None,
        });
    }
    let credential = auth::load_credential(ctx)?;
    let client = ImagineClient::new(
        &ctx.config.base_url,
        credential,
        Duration::from_secs(ctx.config.timeout),
    )?;
    let started = client.start_video(&request).await?;
    let task = if request.wait {
        client
            .wait_video(
                &started.request_id,
                Duration::from_secs(request.poll_interval_secs),
            )
            .await?
    } else {
        started
    };
    finish(&client, task, download).await
}

pub async fn task_get(ctx: &AuthContext, request_id: &str) -> Result<VideoTask> {
    let credential = auth::load_credential(ctx)?;
    let client = ImagineClient::new(
        &ctx.config.base_url,
        credential,
        Duration::from_secs(ctx.config.timeout),
    )?;
    client.get_video(request_id).await
}

pub async fn download(
    ctx: &AuthContext,
    file_id: &str,
    out: &PathBuf,
) -> Result<VideoGenerateResult> {
    let credential = auth::load_credential(ctx)?;
    let client = ImagineClient::new(
        &ctx.config.base_url,
        credential,
        Duration::from_secs(ctx.config.timeout),
    )?;
    let task = if file_id.starts_with("http://") || file_id.starts_with("https://") {
        VideoTask {
            request_id: file_id.to_string(),
            status: "done".to_string(),
            video: Some(crate::model::media::VideoAsset {
                url: Some(file_id.to_string()),
            }),
            error: None,
        }
    } else {
        client.get_video(file_id).await?
    };
    finish(&client, task, Some(out.clone())).await
}

pub async fn list_voices(ctx: &AuthContext) -> Result<serde_json::Value> {
    let credential = auth::load_credential(ctx)?;
    let client = ImagineClient::new(
        &ctx.config.base_url,
        credential,
        Duration::from_secs(ctx.config.timeout),
    )?;
    client.list_voices().await
}

async fn finish(
    client: &ImagineClient,
    task: VideoTask,
    download: Option<PathBuf>,
) -> Result<VideoGenerateResult> {
    let mut path = None;
    if let Some(dest) = download {
        let dest = abs(dest)?.to_path_buf();
        let url = task
            .video
            .as_ref()
            .and_then(|video| video.url.clone())
            .ok_or_else(|| Error::message("video is not ready to download"))?;
        client.download(&url, &dest).await?;
        path = Some(dest.display().to_string());
    }
    Ok(video_result(task, path))
}
