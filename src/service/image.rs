use std::path::PathBuf;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::infra::imagine::{ImagineClient, write_base64_image};
use crate::infra::paths::abs;
use crate::model::media::{ImageGenerateRequest, ImageGenerateResult};
use crate::service::auth::{self, AuthContext};

pub struct ImageGenerateOpts {
    pub request: ImageGenerateRequest,
    pub out: Option<PathBuf>,
    pub out_dir: Option<PathBuf>,
    pub out_prefix: String,
    pub dry_run: bool,
}

pub async fn generate(ctx: &AuthContext, opts: ImageGenerateOpts) -> Result<ImageGenerateResult> {
    if opts.request.n == 0 || opts.request.n > 10 {
        return Err(Error::InvalidValue {
            flag: "n",
            value: opts.request.n.to_string(),
        });
    }
    if opts.out.is_some() && opts.request.n != 1 {
        return Err(Error::message("--out requires --n 1"));
    }
    if opts.dry_run {
        return Ok(ImageGenerateResult {
            model: opts.request.model,
            images: Vec::new(),
        });
    }

    let credential = auth::load_credential(ctx)?;
    let config = &ctx.config;
    let client = ImagineClient::new(
        &config.base_url,
        credential,
        Duration::from_secs(config.timeout),
    )?;
    let mut result = client.generate_image(&opts.request).await?;

    for (index, image) in result.images.iter_mut().enumerate() {
        let dest = output_path(&opts, index)?;
        if let Some(dest) = dest {
            if let Some(b64) = image.b64_json.as_deref() {
                write_base64_image(b64, &dest)?;
            } else if let Some(url) = image.url.as_deref() {
                client.download(url, &dest).await?;
            }
            image.path = Some(dest.display().to_string());
        }
    }
    Ok(result)
}

fn output_path(opts: &ImageGenerateOpts, index: usize) -> Result<Option<PathBuf>> {
    if let Some(out) = &opts.out {
        return Ok(Some(abs(out)?.to_path_buf()));
    }
    if let Some(dir) = &opts.out_dir {
        let dir = abs(dir)?.to_path_buf();
        std::fs::create_dir_all(&dir).map_err(|source| Error::io(&dir, source))?;
        return Ok(Some(dir.join(format!("{}-{index}.jpg", opts.out_prefix))));
    }
    Ok(None)
}
