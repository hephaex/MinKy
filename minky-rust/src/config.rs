use anyhow::Result;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

/// Application configuration loaded from environment variables
#[derive(Clone, Deserialize)]
pub struct Config {
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
