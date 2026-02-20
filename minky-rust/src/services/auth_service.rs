use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{config::Config, error::AppError, models::User};

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32,       // user id
    pub email: String,
    pub role: String,
    pub exp: i64,       // expiration timestamp
    pub iat: i64,       // issued at timestamp
}

/// Authentication service
pub struct AuthService {
    db: PgPool,
    config: Config,
}

impl AuthService {
    pub fn new(db: PgPool, config: Config) -> Self {
        Self { db, config }
    }

    /// Hash a password using Argon2
    pub fn hash_password(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?;
        Ok(hash.to_string())
    }

    /// Verify a password against a hash
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| anyhow::anyhow!("Invalid password hash: {}", e))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Generate JWT access token
    pub fn generate_access_token(&self, user: &User) -> Result<String> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::hours(self.config.jwt_expiration_hours))
            .expect("valid timestamp")
            .timestamp();

        let claims = Claims {
            sub: user.id,
            email: user.email.clone(),
            role: format!("{:?}", user.role).to_lowercase(),
            exp: expiration,
            iat: Utc::now().timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.expose_secret().as_bytes()),
        )?;

        Ok(token)
    }

    /// Generate JWT refresh token (longer expiration)
    pub fn generate_refresh_token(&self, user: &User) -> Result<String> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::days(7))
            .expect("valid timestamp")
            .timestamp();

        let claims = Claims {
            sub: user.id,
            email: user.email.clone(),
            role: format!("{:?}", user.role).to_lowercase(),
            exp: expiration,
            iat: Utc::now().timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.expose_secret().as_bytes()),
        )?;

        Ok(token)
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<Claims, AppError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.expose_secret().as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized)?;

        Ok(token_data.claims)
    }

    /// Find user by email
    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.db)
            .await?;

        Ok(user)
    }

    /// Record failed login attempt
    pub async fn record_failed_login(&self, user_id: i32) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET failed_login_attempts = failed_login_attempts + 1,
                locked_until = CASE
                    WHEN failed_login_attempts >= 4 THEN NOW() + INTERVAL '15 minutes'
                    ELSE locked_until
                END,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Reset login attempts on successful login
    pub async fn reset_login_attempts(&self, user_id: i32) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET failed_login_attempts = 0,
                locked_until = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::UserRole;
    use chrono::Utc;
    use secrecy::SecretString;

    /// Build a test Config without needing a real database
    fn make_config() -> Config {
        Config {
            environment: "test".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            database_url: "postgres://localhost/test_db".to_string(),
            database_max_connections: 1,
            jwt_secret: SecretString::from("test-secret-key-for-unit-tests"),
            jwt_expiration_hours: 24,
            opensearch_url: None,
            openai_api_key: None,
            anthropic_api_key: None,
            git_repo_path: None,
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_signing_secret: None,
        }
    }

    /// Build a minimal User fixture for token tests
    fn make_user() -> User {
        User {
            id: 42,
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            password_hash: String::new(),
            role: UserRole::User,
            is_active: true,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_hash_password_produces_argon2_hash() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let svc = AuthService::new(pool, make_config());
        let hash = svc.hash_password("secret123").expect("hashing should succeed");
        // Argon2 PHC string format starts with "$argon2"
        assert!(hash.starts_with("$argon2"), "Hash should be Argon2 PHC format");
    }

    #[tokio::test]
    async fn test_hash_password_different_salts_produce_different_hashes() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let svc = AuthService::new(pool, make_config());
        let hash1 = svc.hash_password("same_password").unwrap();
        let hash2 = svc.hash_password("same_password").unwrap();
        // Argon2 uses random salt, so hashes must differ
        assert_ne!(hash1, hash2, "Different salts must produce different hashes");
    }

    #[tokio::test]
    async fn test_verify_password_correct_password_returns_true() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let svc = AuthService::new(pool, make_config());
        let hash = svc.hash_password("correct_horse").unwrap();
        let result = svc.verify_password("correct_horse", &hash).unwrap();
        assert!(result, "Correct password should verify successfully");
    }

    #[tokio::test]
    async fn test_verify_password_wrong_password_returns_false() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let svc = AuthService::new(pool, make_config());
        let hash = svc.hash_password("correct_horse").unwrap();
        let result = svc.verify_password("wrong_password", &hash).unwrap();
        assert!(!result, "Wrong password should not verify");
    }

    #[tokio::test]
    async fn test_verify_password_invalid_hash_returns_error() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let svc = AuthService::new(pool, make_config());
        let result = svc.verify_password("password", "not-a-valid-hash");
        assert!(result.is_err(), "Invalid hash string should return error");
    }

    #[tokio::test]
    async fn test_generate_access_token_produces_jwt() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let svc = AuthService::new(pool, make_config());
        let user = make_user();
        let token = svc.generate_access_token(&user).unwrap();
        // JWT has three dot-separated parts
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT should have header.payload.signature");
    }

    #[tokio::test]
    async fn test_validate_token_round_trips_claims() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let svc = AuthService::new(pool, make_config());
        let user = make_user();
        let token = svc.generate_access_token(&user).unwrap();
        let claims = svc.validate_token(&token).expect("Token should be valid");
        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.email, user.email);
        assert_eq!(claims.role, "user");
    }

    #[tokio::test]
    async fn test_validate_token_wrong_secret_returns_unauthorized() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let svc = AuthService::new(pool, make_config());
        let user = make_user();
        let token = svc.generate_access_token(&user).unwrap();

        // Build a service with different secret
        let other_pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let other_config = Config {
            environment: "test".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            database_url: "postgres://localhost/test_db".to_string(),
            database_max_connections: 1,
            jwt_secret: SecretString::from("different-secret-key"),
            jwt_expiration_hours: 24,
            opensearch_url: None,
            openai_api_key: None,
            anthropic_api_key: None,
            git_repo_path: None,
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_signing_secret: None,
        };
        let other_svc = AuthService::new(other_pool, other_config);

        let result = other_svc.validate_token(&token);
        assert!(result.is_err(), "Token signed with different key should be rejected");
    }

    #[tokio::test]
    async fn test_generate_refresh_token_has_longer_expiry_than_access_token() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let svc = AuthService::new(pool, make_config());
        let user = make_user();
        let access = svc.generate_access_token(&user).unwrap();
        let refresh = svc.generate_refresh_token(&user).unwrap();

        let access_claims = svc.validate_token(&access).unwrap();
        let refresh_claims = svc.validate_token(&refresh).unwrap();

        assert!(
            refresh_claims.exp > access_claims.exp,
            "Refresh token should expire later than access token"
        );
    }

    #[tokio::test]
    async fn test_access_token_encodes_admin_role() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let config = Config {
            environment: "test".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            database_url: "postgres://localhost/test_db".to_string(),
            database_max_connections: 1,
            jwt_secret: SecretString::from("test-secret-key-for-unit-tests"),
            jwt_expiration_hours: 24,
            opensearch_url: None,
            openai_api_key: None,
            anthropic_api_key: None,
            git_repo_path: None,
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_signing_secret: None,
        };
        let svc = AuthService::new(pool, config);

        let admin_user = User {
            id: 1,
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password_hash: String::new(),
            role: UserRole::Admin,
            is_active: true,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let token = svc.generate_access_token(&admin_user).unwrap();
        let claims = svc.validate_token(&token).unwrap();
        assert_eq!(claims.role, "admin");
    }
}
