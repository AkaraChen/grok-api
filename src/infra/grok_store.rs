use std::collections::BTreeMap;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serde::{Deserialize, Serialize};
use crate::error::{Error, Result};
use crate::model::auth::{AuthMode, AuthSource, Credential};

/// Official `auth.json` shape used by Grok Build (`~/.grok/auth.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthStore(BTreeMap<String, AuthRecord>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRecord {
    pub key: String,
    #[serde(default)]
    pub auth_mode: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

pub fn load_store(path: &Path) -> Result<Option<AuthStore>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|source| Error::io(path, source))?;
    Ok(Some(serde_json::from_str(&raw)?))
}

pub fn write_store(path: &Path, store: &AuthStore) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    }
    let raw = serde_json::to_string_pretty(store)?;
    fs::write(path, raw).map_err(|source| Error::io(path, source))?;
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path)
            .map_err(|source| Error::io(path, source))?
            .permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions).map_err(|source| Error::io(path, source))?;
    }
    Ok(())
}

pub fn delete_store(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(path).map_err(|source| Error::io(path, source))?;
    Ok(true)
}

pub fn first_credential(store: &AuthStore, source: AuthSource) -> Option<Credential> {
    store.0.values().next().map(|record| Credential {
        source,
        mode: parse_mode(record.auth_mode.as_deref()),
        token: record.key.clone(),
        email: record.email.clone(),
        user_id: record.user_id.clone(),
        expires_at: record.expires_at.clone(),
    })
}

pub fn api_key_store(api_key: &str) -> AuthStore {
    let mut store = AuthStore::default();
    store.0.insert(
        "api_key".to_string(),
        AuthRecord {
            key: api_key.to_string(),
            auth_mode: Some("api_key".to_string()),
            email: None,
            user_id: None,
            expires_at: None,
            refresh_token: None,
        },
    );
    store
}

fn parse_mode(value: Option<&str>) -> AuthMode {
    match value {
        Some("oidc") | Some("oauth") => AuthMode::Oidc,
        Some("api_key") => AuthMode::ApiKey,
        Some("external") => AuthMode::External,
        _ => AuthMode::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_store() -> serde_json::Value {
        serde_json::json!({
            "https://auth.x.ai::example": {
                "key": "xai-test-token-abcdef",
                "auth_mode": "oidc",
                "email": "user@example.com",
                "user_id": "u_1",
                "expires_at": "2026-09-01T00:00:00Z"
            }
        })
    }

    #[test]
    fn extracts_first_official_record() {
        let store: AuthStore = serde_json::from_value(sample_store()).unwrap();
        let credential = first_credential(&store, AuthSource::GrokCli).unwrap();
        assert_eq!(credential.token, "xai-test-token-abcdef");
        assert_eq!(credential.email.as_deref(), Some("user@example.com"));
        assert_eq!(credential.mode, AuthMode::Oidc);
        assert_eq!(credential.source, AuthSource::GrokCli);
    }

    #[test]
    fn write_and_reload_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("auth.json");
        write_store(&path, &api_key_store("xai-secret")).unwrap();
        let loaded = load_store(&path).unwrap().unwrap();
        let credential = first_credential(&loaded, AuthSource::GrokApi).unwrap();
        assert_eq!(credential.token, "xai-secret");
        assert_eq!(credential.mode, AuthMode::ApiKey);
    }
}
