use anyhow::Result;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

/// Application configuration loaded from environment variables
#[derive(Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_environment")]
    pub environment: String,

    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    pub database_url: String,

    #[serde(default = "default_max_connections")]
    pub database_max_connections: u32,

    pub jwt_secret: SecretString,

    #[serde(default = "default_jwt_expiration")]
    pub jwt_expiration_hours: i64,

    pub opensearch_url: Option<String>,

    pub openai_api_key: Option<SecretString>,

    pub anthropic_api_key: Option<SecretString>,

    pub git_repo_path: Option<String>,

    /// Slack app client ID (for OAuth 2.0 flow)
    pub slack_client_id: Option<String>,

    /// Slack app client secret (for OAuth 2.0 token exchange)
    pub slack_client_secret: Option<SecretString>,

    /// Slack OAuth redirect URI
    pub slack_redirect_uri: Option<String>,

    /// Slack signing secret (for webhook signature verification)
    pub slack_signing_secret: Option<SecretString>,

    /// CORS allowed origins (comma-separated, e.g., "http://localhost:3000,https://minky.example.com")
    #[serde(default = "default_cors_origins")]
    pub cors_allowed_origins: String,
}

fn default_cors_origins() -> String {
    "http://localhost:3000,http://127.0.0.1:3000".to_string()
}

fn default_environment() -> String {
    "development".to_string()
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8000
}

fn default_max_connections() -> u32 {
    10
}

fn default_jwt_expiration() -> i64 {
    24
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let config = config::Config::builder()
            .add_source(config::Environment::default())
            .build()?;

        Ok(config.try_deserialize()?)
    }

    /// Get JWT secret as bytes for signing
    pub fn jwt_secret_bytes(&self) -> &[u8] {
        self.jwt_secret.expose_secret().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(secret: &str) -> Config {
        Config {
            environment: "development".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            database_url: "postgres://localhost/test".to_string(),
            database_max_connections: 5,
            jwt_secret: SecretString::from(secret),
            jwt_expiration_hours: 24,
            opensearch_url: None,
            openai_api_key: None,
            anthropic_api_key: None,
            git_repo_path: None,
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_signing_secret: None,
            cors_allowed_origins: "http://localhost:3000".to_string(),
        }
    }

    #[test]
    fn test_jwt_secret_bytes_matches_original_string() {
        let config = make_config("mysecret");
        assert_eq!(config.jwt_secret_bytes(), b"mysecret");
    }

    #[test]
    fn test_jwt_secret_bytes_non_empty() {
        let config = make_config("some-secret-key");
        assert!(!config.jwt_secret_bytes().is_empty());
    }

    #[test]
    fn test_jwt_secret_bytes_length_matches_string_len() {
        let secret = "hello-world-secret";
        let config = make_config(secret);
        assert_eq!(config.jwt_secret_bytes().len(), secret.len());
    }

    #[test]
    fn test_slack_client_id_defaults_to_none() {
        let config = make_config("secret");
        assert!(config.slack_client_id.is_none());
    }

    #[test]
    fn test_slack_client_secret_defaults_to_none() {
        let config = make_config("secret");
        assert!(config.slack_client_secret.is_none());
    }

    #[test]
    fn test_slack_redirect_uri_defaults_to_none() {
        let config = make_config("secret");
        assert!(config.slack_redirect_uri.is_none());
    }

    #[test]
    fn test_slack_signing_secret_defaults_to_none() {
        let config = make_config("secret");
        assert!(config.slack_signing_secret.is_none());
    }

    #[test]
    fn test_config_with_slack_client_id() {
        let mut config = make_config("secret");
        config.slack_client_id = Some("my-app-id".to_string());
        assert_eq!(config.slack_client_id.as_deref(), Some("my-app-id"));
    }

    #[test]
    fn test_config_with_slack_redirect_uri() {
        let mut config = make_config("secret");
        config.slack_redirect_uri = Some("https://example.com/callback".to_string());
        assert_eq!(
            config.slack_redirect_uri.as_deref(),
            Some("https://example.com/callback")
        );
    }

    #[test]
    fn test_config_slack_client_id_is_none_when_not_set() {
        let config = make_config("test");
        // Ensure slack fields do not interfere with jwt_secret_bytes
        assert_eq!(config.jwt_secret_bytes(), b"test");
        assert!(config.slack_client_id.is_none());
    }

    #[test]
    fn test_config_all_optional_fields_can_be_none() {
        let config = make_config("s");
        assert!(config.opensearch_url.is_none());
        assert!(config.openai_api_key.is_none());
        assert!(config.anthropic_api_key.is_none());
        assert!(config.git_repo_path.is_none());
        assert!(config.slack_client_id.is_none());
        assert!(config.slack_client_secret.is_none());
        assert!(config.slack_redirect_uri.is_none());
        assert!(config.slack_signing_secret.is_none());
    }

    #[test]
    fn test_config_default_port_is_8000() {
        let config = make_config("s");
        assert_eq!(config.port, 8000);
    }

    #[test]
    fn test_cors_allowed_origins_can_be_parsed() {
        let config = make_config("s");
        let origins: Vec<&str> = config.cors_allowed_origins.split(',').collect();
        assert!(!origins.is_empty());
    }

    #[test]
    fn test_default_cors_origins() {
        let default = default_cors_origins();
        assert!(default.contains("localhost:3000"));
    }
}
