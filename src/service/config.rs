use std::fs;

use crate::error::{Error, Result};
use crate::infra::paths::{ensure_media_home, media_config_path};
use crate::model::config::AppConfig;

pub fn load() -> Result<AppConfig> {
    let path = media_config_path();
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = fs::read_to_string(&path).map_err(|source| Error::io(&path, source))?;
    Ok(toml::from_str(&raw)?)
}

pub fn save(config: &AppConfig) -> Result<()> {
    ensure_media_home()?;
    let path = media_config_path();
    let raw =
        toml::to_string_pretty(config).map_err(|err| Error::Config(err.to_string()))?;
    fs::write(&path, raw).map_err(|source| Error::io(&path, source))?;
    Ok(())
}

pub fn set(key: &str, value: &str) -> Result<AppConfig> {
    let mut config = load()?;
    config
        .set(key, value)
        .map_err(|flag| Error::InvalidValue {
            flag,
            value: value.to_string(),
        })?;
    save(&config)?;
    Ok(config)
}
