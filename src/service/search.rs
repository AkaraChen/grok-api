use std::time::Duration;

use crate::error::Result;
use crate::infra::imagine::ImagineClient;
use crate::model::search::{WebSearchRequest, WebSearchResult, XSearchRequest, XSearchResult};
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
    let client = client(ctx)?;
    crate::infra::search::search(&client, &request).await
}

pub async fn x_query(
    ctx: &AuthContext,
    request: XSearchRequest,
    dry_run: bool,
) -> Result<XSearchResult> {
    if dry_run {
        return Ok(XSearchResult {
            query: request.query,
            content: String::new(),
            citations: Vec::new(),
            from_date: request.from_date,
            to_date: request.to_date,
            allowed_x_handles: request.allowed_x_handles,
            excluded_x_handles: request.excluded_x_handles,
        });
    }
    let client = client(ctx)?;
    crate::infra::search::x_search(&client, &request).await
}

fn client(ctx: &AuthContext) -> Result<ImagineClient> {
    let credential = auth::load_credential(ctx)?;
    ImagineClient::new(
        &ctx.config.base_url,
        credential,
        Duration::from_secs(ctx.config.timeout),
    )
}
