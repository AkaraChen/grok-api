use std::time::Duration;

use crate::error::Result;
use crate::infra::imagine::ImagineClient;
use crate::model::search::{WebSearchRequest, WebSearchResult};
use crate::service::auth::{self, AuthContext};

pub async fn query(
    ctx: &AuthContext,
    request: WebSearchRequest,
    dry_run: bool,
) -> Result<WebSearchResult> {
    if dry_run {
        return Ok(WebSearchResult {
            query: request.query,
            content: String::new(),
            citations: Vec::new(),
            allowed_domains: request.allowed_domains,
        });
    }
    let credential = auth::load_credential(ctx)?;
    let client = ImagineClient::new(
        &ctx.config.base_url,
        credential,
        Duration::from_secs(ctx.config.timeout),
    )?;
    crate::infra::search::search(&client, &request).await
}
