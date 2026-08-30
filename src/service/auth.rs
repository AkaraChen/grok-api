use std::process::Stdio;

use crate::error::{Error, Result};
use crate::infra::grok_store::{
    api_key_store, delete_store, first_credential, load_store, write_store,
};
use crate::infra::paths::{
    api_auth_path, api_home, ensure_api_home, grok_cli_auth_path, grok_cli_home,
};
use crate::model::auth::{AuthSource, AuthStatus, Credential};
use crate::model::config::AppConfig;
use crate::service::config as config_service;

pub struct AuthContext {
    pub config: AppConfig,
    pub api_key_override: Option<String>,
    pub source_override: Option<AuthSource>,
}

pub fn resolve_source(ctx: &AuthContext) -> AuthSource {
    ctx.source_override.unwrap_or(ctx.config.auth_source)
}

pub fn home_for(source: AuthSource) -> std::path::PathBuf {
    match source {
        AuthSource::GrokApi => api_home(),
        AuthSource::GrokCli => grok_cli_home(),
    }
}

pub fn auth_path_for(source: AuthSource) -> std::path::PathBuf {
    match source {
        AuthSource::GrokApi => api_auth_path(),
        AuthSource::GrokCli => grok_cli_auth_path(),
    }
}

pub fn load_credential(ctx: &AuthContext) -> Result<Credential> {
    let env_key = std::env::var("XAI_API_KEY")
        .ok()
        .filter(|value| !value.is_empty());
    if let Some(api_key) = ctx
        .api_key_override
        .as_ref()
        .or(ctx.config.api_key.as_ref())
        .or(env_key.as_ref())
    {
        return Ok(Credential {
            source: AuthSource::GrokApi,
            mode: crate::model::auth::AuthMode::ApiKey,
            token: api_key.clone(),
            email: None,
            user_id: None,
            expires_at: None,
        });
    }

    let source = resolve_source(ctx);
    if let Some(credential) = credential_from(source)? {
        return Ok(credential);
    }
    if source == AuthSource::GrokApi {
        if let Some(credential) = credential_from(AuthSource::GrokCli)? {
            return Ok(credential);
        }
    }
    Err(Error::auth(
        "no credential found. Run `grok-api auth login --from-grok-cli` or `grok-api auth login --oauth`",
    ))
}

fn credential_from(source: AuthSource) -> Result<Option<Credential>> {
    let path = auth_path_for(source);
    Ok(load_store(&path)?.and_then(|store| first_credential(&store, source)))
}

pub fn status(ctx: &AuthContext) -> Result<AuthStatus> {
    match load_credential(ctx) {
        Ok(credential) => Ok(AuthStatus::from_credential(
            &credential,
            home_for(credential.source).display().to_string(),
        )),
        Err(_) => Ok(AuthStatus::empty(
            resolve_source(ctx),
            home_for(resolve_source(ctx)).display().to_string(),
        )),
    }
}

pub fn login_from_grok_cli() -> Result<AuthStatus> {
    let path = grok_cli_auth_path();
    let store = load_store(&path)?.ok_or_else(|| {
        Error::auth(format!(
            "no Grok CLI auth at {}. Run `grok login` first.",
            path.display()
        ))
    })?;
    let credential = first_credential(&store, AuthSource::GrokCli)
        .ok_or_else(|| Error::auth("Grok CLI auth.json has no usable credential"))?;
    let mut config = config_service::load()?;
    config.auth_source = AuthSource::GrokCli;
    config_service::save(&config)?;
    Ok(AuthStatus::from_credential(
        &credential,
        grok_cli_home().display().to_string(),
    ))
}

pub fn login_with_api_key(api_key: &str) -> Result<AuthStatus> {
    ensure_api_home()?;
    write_store(&api_auth_path(), &api_key_store(api_key))?;
    let mut config = config_service::load()?;
    config.auth_source = AuthSource::GrokApi;
    config_service::save(&config)?;
    let store = load_store(&api_auth_path())?.expect("just written");
    let credential = first_credential(&store, AuthSource::GrokApi).expect("api key record");
    Ok(AuthStatus::from_credential(
        &credential,
        api_home().display().to_string(),
    ))
}

pub fn login_with_official_grok(device_auth: bool) -> Result<AuthStatus> {
    let home = ensure_api_home()?;
    let mut command = std::process::Command::new("grok");
    command.arg("login");
    if device_auth {
        command.arg("--device-auth");
    } else {
        command.arg("--oauth");
    }
    command.env("GROK_HOME", &home);
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    let status = command
        .status()
        .map_err(|source| Error::io("grok", source))?;
    if !status.success() {
        return Err(Error::message(format!(
            "official `grok login` exited with {status}"
        )));
    }
    let mut config = config_service::load()?;
    config.auth_source = AuthSource::GrokApi;
    config_service::save(&config)?;
    let store = load_store(&api_auth_path())?.ok_or_else(|| {
        Error::auth(format!(
            "official login finished but {} is missing",
            api_auth_path().display()
        ))
    })?;
    let credential = first_credential(&store, AuthSource::GrokApi)
        .ok_or_else(|| Error::auth("official login wrote an empty auth store"))?;
    Ok(AuthStatus::from_credential(
        &credential,
        home.display().to_string(),
    ))
}

pub fn refresh(device_auth: bool) -> Result<AuthStatus> {
    login_with_official_grok(device_auth)
}

pub fn logout(yes: bool) -> Result<bool> {
    if !yes {
        return Err(Error::message(
            "refusing to logout without --yes (this only clears ~/.grok-api, never ~/.grok)",
        ));
    }
    delete_store(&api_auth_path())
}
