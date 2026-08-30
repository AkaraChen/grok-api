use std::path::{Path, PathBuf};

use xai_grok_paths::AbsPathBuf;

use crate::error::{Error, Result};

const API_DIR_NAME: &str = ".grok-api";

/// Official Grok CLI home: `~/.grok`, ignoring `$GROK_HOME`.
pub fn grok_cli_home() -> PathBuf {
    xai_dirs::default_grok_home()
}

/// This CLI's home. Official login uses `$GROK_HOME`; we point that at this path.
pub fn api_home() -> PathBuf {
    xai_dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(API_DIR_NAME)
}

pub fn api_auth_path() -> PathBuf {
    api_home().join("auth.json")
}

pub fn grok_cli_auth_path() -> PathBuf {
    grok_cli_home().join("auth.json")
}

pub fn api_config_path() -> PathBuf {
    api_home().join("config.toml")
}

pub fn ensure_api_home() -> Result<PathBuf> {
    let home = api_home();
    std::fs::create_dir_all(&home).map_err(|source| Error::io(&home, source))?;
    Ok(home)
}

pub fn abs(path: impl AsRef<Path>) -> Result<AbsPathBuf> {
    let path = path.as_ref();
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|source| Error::io(".", source))?
            .join(path)
    };
    Ok(AbsPathBuf::new(resolved)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grok_cli_home_ends_with_grok() {
        assert!(grok_cli_home().ends_with(".grok"));
    }

    #[test]
    fn api_home_ends_with_grok_api() {
        assert!(api_home().ends_with(".grok-api"));
    }

    #[test]
    fn homes_are_siblings() {
        assert_ne!(grok_cli_home(), api_home());
        assert_eq!(grok_cli_home().parent(), api_home().parent());
    }
}
