//! Slack OAuth 2.0 token exchange service.
//!
//! Handles:
//!   1. `oauth.v2.access` API call with a temporary `code`
//!   2. Persisting workspace credentials to `platform_configs` table
//!   3. Retrieving stored credentials by workspace ID

use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;

// ---------------------------------------------------------------------------
// Slack API response types
// ---------------------------------------------------------------------------

/// Response from Slack's `oauth.v2.access` endpoint.
#[derive(Debug, Deserialize)]
pub struct SlackOAuthResponse {
    pub ok: bool,
    pub error: Option<String>,
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub bot_user_id: Option<String>,
    pub app_id: Option<String>,
    pub team: Option<SlackTeam>,
    pub authed_user: Option<SlackAuthedUser>,
}

/// Slack team/workspace info from OAuth response.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SlackTeam {
    pub id: String,
    pub name: String,
}

/// Authenticated user info from OAuth response.
#[derive(Debug, Deserialize)]
pub struct SlackAuthedUser {
    pub id: String,
    pub scope: Option<String>,
}

/// Stored workspace credentials returned after DB write.
#[derive(Debug, Serialize, Clone)]
pub struct WorkspaceCredentials {
    pub platform_config_id: i32,
    pub workspace_id: String,
    pub workspace_name: String,
    pub is_active: bool,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Runtime configuration for the OAuth service.
#[derive(Debug, Clone)]
pub struct SlackOAuthConfig {
    /// Slack app client ID.
    pub client_id: String,
    /// Slack app client secret.
    pub client_secret: String,
    /// Redirect URI registered with the Slack app.
    pub redirect_uri: Option<String>,
}

impl SlackOAuthConfig {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            redirect_uri: None,
        }
    }

    pub fn with_redirect_uri(mut self, uri: impl Into<String>) -> Self {
        self.redirect_uri = Some(uri.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// Handles Slack OAuth 2.0 token exchange and credential persistence.
pub struct SlackOAuthService {
    http: Client,
    config: SlackOAuthConfig,
}

impl SlackOAuthService {
    pub fn new(config: SlackOAuthConfig) -> Self {
        Self {
            http: Client::new(),
            config,
        }
    }

    /// Exchange a temporary OAuth `code` for a bot token.
    ///
    /// Calls `https://slack.com/api/oauth.v2.access` and returns the parsed
    /// Slack response on success.
    pub async fn exchange_code(&self, code: &str) -> Result<SlackOAuthResponse, AppError> {
        let mut params = vec![
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.as_str()),
            ("code", code),
        ];

        let redirect_uri_str;
        if let Some(ref uri) = self.config.redirect_uri {
            redirect_uri_str = uri.clone();
            params.push(("redirect_uri", &redirect_uri_str));
        }

        let response = self
            .http
            .post("https://slack.com/api/oauth.v2.access")
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Slack OAuth request failed: {e}"))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Slack OAuth HTTP error {status}: {body}"
            )));
        }

        let oauth_resp: SlackOAuthResponse = response
            .json()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Failed to parse Slack OAuth response: {e}"))
            })?;

        if !oauth_resp.ok {
            let err = oauth_resp.error.as_deref().unwrap_or("unknown");
            return Err(AppError::ExternalService(format!(
                "Slack OAuth error: {err}"
            )));
        }

        Ok(oauth_resp)
    }

    /// Persist workspace credentials to the `platform_configs` table.
    ///
    /// Uses `INSERT ... ON CONFLICT DO UPDATE` (upsert) so re-installing the
    /// app to the same workspace refreshes the token.
    pub async fn save_workspace_credentials(
        pool: &PgPool,
        oauth_resp: &SlackOAuthResponse,
    ) -> Result<WorkspaceCredentials, AppError> {
        let team = oauth_resp.team.as_ref().ok_or_else(|| {
            AppError::ExternalService("Slack OAuth response missing team info".to_string())
        })?;

        let bot_token = oauth_resp.access_token.as_deref().ok_or_else(|| {
            AppError::ExternalService("Slack OAuth response missing access_token".to_string())
        })?;

        let row = sqlx::query(
            r#"
            INSERT INTO platform_configs
                (platform, workspace_id, workspace_name, bot_token, is_active)
            VALUES
                ('slack', $1, $2, $3, TRUE)
            ON CONFLICT (platform, workspace_id)
            DO UPDATE SET
                workspace_name = EXCLUDED.workspace_name,
                bot_token      = EXCLUDED.bot_token,
                is_active      = TRUE,
                updated_at     = NOW()
            RETURNING id, workspace_id, workspace_name, is_active
            "#,
        )
        .bind(&team.id)
        .bind(&team.name)
        .bind(bot_token)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        use sqlx::Row;
        Ok(WorkspaceCredentials {
            platform_config_id: row.get::<i32, _>("id"),
            workspace_id: row.get::<String, _>("workspace_id"),
            workspace_name: row.get::<String, _>("workspace_name"),
            is_active: row.get::<bool, _>("is_active"),
        })
    }

    /// Retrieve stored credentials for a workspace.
    pub async fn get_workspace_credentials(
        pool: &PgPool,
        workspace_id: &str,
    ) -> Result<Option<WorkspaceCredentials>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, workspace_name, is_active
            FROM platform_configs
            WHERE platform = 'slack' AND workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        use sqlx::Row;
        Ok(row.map(|r| WorkspaceCredentials {
            platform_config_id: r.get::<i32, _>("id"),
            workspace_id: r.get::<String, _>("workspace_id"),
            workspace_name: r.get::<String, _>("workspace_name"),
            is_active: r.get::<bool, _>("is_active"),
        }))
    }

    /// Build the Slack OAuth authorization URL to redirect users to.
    pub fn build_auth_url(
        client_id: &str,
        scopes: &[&str],
        redirect_uri: Option<&str>,
        state: Option<&str>,
    ) -> String {
        let scope = scopes.join(",");
        let mut url = format!(
            "https://slack.com/oauth/v2/authorize?client_id={}&scope={}",
            client_id, scope
        );
        if let Some(uri) = redirect_uri {
            // Percent-encode the redirect URI (replace common chars)
            let encoded = uri.replace(':', "%3A").replace('/', "%2F");
            url.push_str(&format!("&redirect_uri={}", encoded));
        }
        if let Some(s) = state {
            url.push_str(&format!("&state={}", s));
        }
        url
    }

    /// Validate that the OAuth `state` parameter matches the one we sent.
    /// Returns an error if state is missing or does not match.
    pub fn validate_state(
        received_state: Option<&str>,
        expected_state: &str,
    ) -> Result<(), AppError> {
        match received_state {
            None => Err(AppError::Validation(
                "Missing OAuth state parameter".to_string(),
            )),
            Some(s) if s != expected_state => Err(AppError::Validation(
                "OAuth state mismatch – possible CSRF attack".to_string(),
            )),
            _ => Ok(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests (pure logic – no I/O)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> SlackOAuthConfig {
        SlackOAuthConfig::new("client-id-123", "client-secret-456")
    }

    #[test]
    fn test_config_stores_client_id_and_secret() {
        let cfg = make_config();
        assert_eq!(cfg.client_id, "client-id-123");
        assert_eq!(cfg.client_secret, "client-secret-456");
        assert!(cfg.redirect_uri.is_none());
    }

    #[test]
    fn test_config_with_redirect_uri() {
        let cfg = make_config().with_redirect_uri("https://example.com/oauth/callback");
        assert_eq!(
            cfg.redirect_uri.as_deref(),
            Some("https://example.com/oauth/callback")
        );
    }

    #[test]
    fn test_build_auth_url_contains_client_id() {
        let url = SlackOAuthService::build_auth_url("my-client", &["channels:read"], None, None);
        assert!(url.contains("client_id=my-client"));
    }

    #[test]
    fn test_build_auth_url_contains_scope() {
        let url = SlackOAuthService::build_auth_url(
            "cid",
            &["channels:read", "chat:write"],
            None,
            None,
        );
        assert!(url.contains("channels%3Aread") || url.contains("channels:read"));
        assert!(url.contains("chat"));
    }

    #[test]
    fn test_build_auth_url_with_state() {
        let url = SlackOAuthService::build_auth_url("cid", &["channels:read"], None, Some("xyz"));
        assert!(url.contains("state=xyz"));
    }

    #[test]
    fn test_build_auth_url_without_state() {
        let url = SlackOAuthService::build_auth_url("cid", &["channels:read"], None, None);
        assert!(!url.contains("state="));
    }

    #[test]
    fn test_build_auth_url_with_redirect_uri() {
        let url = SlackOAuthService::build_auth_url(
            "cid",
            &["channels:read"],
            Some("https://example.com/callback"),
            None,
        );
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("example.com"));
    }

    #[test]
    fn test_validate_state_matches() {
        let result = SlackOAuthService::validate_state(Some("abc123"), "abc123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_state_mismatch() {
        let result = SlackOAuthService::validate_state(Some("wrong"), "expected");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("mismatch") || err.to_string().contains("CSRF"));
    }

    #[test]
    fn test_validate_state_missing() {
        let result = SlackOAuthService::validate_state(None, "expected");
        assert!(result.is_err());
    }

    #[test]
    fn test_slack_oauth_response_ok_false_has_error() {
        let json = r#"{"ok": false, "error": "invalid_code"}"#;
        let resp: SlackOAuthResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.ok);
        assert_eq!(resp.error.as_deref(), Some("invalid_code"));
    }

    #[test]
    fn test_slack_oauth_response_ok_true() {
        let json = r#"{
            "ok": true,
            "access_token": "xoxb-test-token",
            "token_type": "bot",
            "scope": "channels:read",
            "team": {"id": "T123", "name": "My Team"}
        }"#;
        let resp: SlackOAuthResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.access_token.as_deref(), Some("xoxb-test-token"));
        let team = resp.team.unwrap();
        assert_eq!(team.id, "T123");
        assert_eq!(team.name, "My Team");
    }

    #[test]
    fn test_slack_team_serde_roundtrip() {
        let team = SlackTeam {
            id: "T456".to_string(),
            name: "Engineering".to_string(),
        };
        let json = serde_json::to_string(&team).unwrap();
        let back: SlackTeam = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "T456");
        assert_eq!(back.name, "Engineering");
    }

    #[test]
    fn test_workspace_credentials_is_active() {
        let creds = WorkspaceCredentials {
            platform_config_id: 1,
            workspace_id: "T789".to_string(),
            workspace_name: "Workspace A".to_string(),
            is_active: true,
        };
        assert!(creds.is_active);
        assert_eq!(creds.workspace_id, "T789");
    }
}
