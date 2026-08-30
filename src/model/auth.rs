use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AuthSource {
    #[default]
    GrokApi,
    GrokCli,
}

impl AuthSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GrokApi => "grok-api",
            Self::GrokCli => "grok-cli",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "grok-api" | "api" => Some(Self::GrokApi),
            "grok-cli" | "grok" => Some(Self::GrokCli),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    Oidc,
    ApiKey,
    External,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Credential {
    pub source: AuthSource,
    pub mode: AuthMode,
    pub token: String,
    pub email: Option<String>,
    pub user_id: Option<String>,
    pub expires_at: Option<String>,
}

impl Credential {
    pub fn is_session(&self) -> bool {
        matches!(self.mode, AuthMode::Oidc | AuthMode::External)
    }

    pub fn redacted_token(&self) -> String {
        let token = self.token.as_str();
        if token.len() <= 8 {
            return "********".to_string();
        }
        format!("{}…{}", &token[..4], &token[token.len() - 4..])
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub source: &'static str,
    pub mode: Option<&'static str>,
    pub email: Option<String>,
    pub user_id: Option<String>,
    pub expires_at: Option<String>,
    pub home: String,
    pub token: Option<String>,
}

impl AuthStatus {
    pub fn from_credential(credential: &Credential, home: String) -> Self {
        Self {
            authenticated: true,
            source: credential.source.as_str(),
            mode: Some(match credential.mode {
                AuthMode::Oidc => "oidc",
                AuthMode::ApiKey => "api_key",
                AuthMode::External => "external",
                AuthMode::Unknown => "unknown",
            }),
            email: credential.email.clone(),
            user_id: credential.user_id.clone(),
            expires_at: credential.expires_at.clone(),
            home,
            token: Some(credential.redacted_token()),
        }
    }

    pub fn empty(source: AuthSource, home: String) -> Self {
        Self {
            authenticated: false,
            source: source.as_str(),
            mode: None,
            email: None,
            user_id: None,
            expires_at: None,
            home,
            token: None,
        }
    }
}
