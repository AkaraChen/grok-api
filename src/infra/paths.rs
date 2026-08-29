use std::path::{Path, PathBuf};

use xai_grok_paths::AbsPathBuf;

use crate::error::{Error, Result};

const MEDIA_DIR_NAME: &str = ".grok-media";

/// Official Grok CLI home: `~/.grok`, ignoring `$GROK_HOME`.
pub fn grok_cli_home() -> PathBuf {
    xai_dirs::default_grok_home()
}

/// This CLI's home. Official login uses `$GROK_HOME`; we point that at this path.
pub fn media_home() -> PathBuf {
    xai_dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(MEDIA_DIR_NAME)
}

pub fn media_auth_path() -> PathBuf {
    media_home().join("auth.json")
}

pub fn grok_cli_auth_path() -> PathBuf {
    grok_cli_home().join("auth.json")
}

pub fn media_config_path() -> PathBuf {
    media_home().join("config.toml")
}

pub fn ensure_media_home() -> Result<PathBuf> {
    let home = media_home();
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
    fn media_home_ends_with_grok_media() {
        assert!(media_home().ends_with(".grok-media"));
    }

    #[test]
    fn homes_are_siblings() {
        assert_ne!(grok_cli_home(), media_home());
        assert_eq!(grok_cli_home().parent(), media_home().parent());
    }
}
