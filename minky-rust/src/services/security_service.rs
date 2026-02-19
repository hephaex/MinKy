use anyhow::Result;
use chrono::{Duration, Utc};
use rand::Rng;
use sqlx::PgPool;

use crate::models::{
    ApiKey, ApiKeyWithSecret, BlockIpRequest, CreateApiKeyRequest, IpBlock, SecurityEvent, SecurityEventType, SecurityReport, SecuritySettings,
    SessionInfo, Severity,
};

/// Raw DB row type for security event queries
type SecurityEventRow = (
    i64,
    String,
    String,
    Option<i32>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<serde_json::Value>,
    chrono::DateTime<chrono::Utc>,
);

/// Raw DB row type for ip block queries
type IpBlockRow = (
    i64,
    String,
    String,
    Option<i32>,
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
    bool,
);

/// Raw DB row type for API key queries
type ApiKeyRow = (
    i64,
    String,
    String,
    i32,
    serde_json::Value,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
    chrono::DateTime<chrono::Utc>,
    bool,
);

/// Raw DB row type for session info queries
type SessionInfoRow = (
    String,
    i32,
    String,
    String,
    Option<String>,
    Option<String>,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
);

/// Security service
pub struct SecurityService {
    db: PgPool,
}

impl SecurityService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Log security event
    #[allow(clippy::too_many_arguments)]
    pub async fn log_event(
        &self,
        event_type: SecurityEventType,
        severity: Severity,
        user_id: Option<i32>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        resource_type: Option<&str>,
        resource_id: Option<&str>,
        details: Option<serde_json::Value>,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO security_events (event_type, severity, user_id, ip_address, user_agent, resource_type, resource_id, details, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            RETURNING id
            "#,
        )
        .bind(serde_json::to_string(&event_type)?)
        .bind(serde_json::to_string(&severity)?)
        .bind(user_id)
        .bind(ip_address)
        .bind(user_agent)
        .bind(resource_type)
        .bind(resource_id)
        .bind(details)
        .fetch_one(&self.db)
        .await?;

        Ok(row.0)
    }

    /// Get security events
    pub async fn get_events(
        &self,
        page: i32,
        limit: i32,
        severity: Option<Severity>,
        event_type: Option<SecurityEventType>,
    ) -> Result<Vec<SecurityEvent>> {
        let offset = (page - 1) * limit;

        let rows: Vec<SecurityEventRow> = sqlx::query_as(
            r#"
            SELECT
                e.id,
                e.event_type,
                e.severity,
                e.user_id,
                u.username,
                e.ip_address,
                e.user_agent,
                e.resource_type,
                e.resource_id,
                e.details,
                e.created_at
            FROM security_events e
            LEFT JOIN users u ON e.user_id = u.id
            WHERE ($1::text IS NULL OR e.severity = $1)
              AND ($2::text IS NULL OR e.event_type = $2)
            ORDER BY e.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(severity.map(|s| serde_json::to_string(&s).unwrap_or_default()))
        .bind(event_type.map(|e| serde_json::to_string(&e).unwrap_or_default()))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SecurityEvent {
                id: r.0,
                event_type: serde_json::from_str(&r.1).unwrap_or(SecurityEventType::SuspiciousActivity),
                severity: serde_json::from_str(&r.2).unwrap_or(Severity::Low),
                user_id: r.3,
                username: r.4,
                ip_address: r.5,
                user_agent: r.6,
                resource_type: r.7,
                resource_id: r.8,
                details: r.9,
                created_at: r.10,
            })
            .collect())
    }

    /// Block IP address
    pub async fn block_ip(&self, user_id: i32, request: BlockIpRequest) -> Result<IpBlock> {
        let expires_at = if request.is_permanent.unwrap_or(false) {
            None
        } else {
            let hours = request.duration_hours.unwrap_or(24);
            Some(Utc::now() + Duration::hours(hours as i64))
        };

        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO ip_blocks (ip_address, reason, blocked_by, blocked_at, expires_at, is_permanent)
            VALUES ($1, $2, $3, NOW(), $4, $5)
            RETURNING id
            "#,
        )
        .bind(&request.ip_address)
        .bind(&request.reason)
        .bind(user_id)
        .bind(expires_at)
        .bind(request.is_permanent.unwrap_or(false))
        .fetch_one(&self.db)
        .await?;

        Ok(IpBlock {
            id: row.0,
            ip_address: request.ip_address,
            reason: request.reason,
            blocked_by: Some(user_id),
            blocked_at: Utc::now(),
            expires_at,
            is_permanent: request.is_permanent.unwrap_or(false),
        })
    }

    /// Unblock IP address
    pub async fn unblock_ip(&self, ip_address: &str) -> Result<()> {
        sqlx::query("DELETE FROM ip_blocks WHERE ip_address = $1")
            .bind(ip_address)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Check if IP is blocked
    pub async fn is_ip_blocked(&self, ip_address: &str) -> Result<bool> {
        let row: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT id FROM ip_blocks
            WHERE ip_address = $1
              AND (is_permanent = true OR expires_at > NOW())
            "#,
        )
        .bind(ip_address)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.is_some())
    }

    /// Get blocked IPs
    pub async fn get_blocked_ips(&self) -> Result<Vec<IpBlock>> {
        let rows: Vec<IpBlockRow> = sqlx::query_as(
            r#"
            SELECT id, ip_address, reason, blocked_by, blocked_at, expires_at, is_permanent
            FROM ip_blocks
            WHERE is_permanent = true OR expires_at > NOW()
            ORDER BY blocked_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| IpBlock {
                id: r.0,
                ip_address: r.1,
                reason: r.2,
                blocked_by: r.3,
                blocked_at: r.4,
                expires_at: r.5,
                is_permanent: r.6,
            })
            .collect())
    }

    /// Create API key
    pub async fn create_api_key(
        &self,
        user_id: i32,
        request: CreateApiKeyRequest,
    ) -> Result<ApiKeyWithSecret> {
        let key = generate_api_key();
        let key_prefix = &key[..8];
        let key_hash = hash_api_key(&key);

        let expires_at = request
            .expires_in_days
            .map(|days| Utc::now() + Duration::days(days as i64));

        let permissions = request.permissions.unwrap_or_default();
        let permissions_json = serde_json::to_value(&permissions)?;

        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO api_keys (name, key_prefix, key_hash, user_id, permissions, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            RETURNING id
            "#,
        )
        .bind(&request.name)
        .bind(key_prefix)
        .bind(&key_hash)
        .bind(user_id)
        .bind(&permissions_json)
        .bind(expires_at)
        .fetch_one(&self.db)
        .await?;

        Ok(ApiKeyWithSecret {
            id: row.0,
            name: request.name,
            key,
            permissions,
            expires_at,
            created_at: Utc::now(),
        })
    }

    /// List API keys for user
    pub async fn list_api_keys(&self, user_id: i32) -> Result<Vec<ApiKey>> {
        let rows: Vec<ApiKeyRow> = sqlx::query_as(
            r#"
            SELECT id, name, key_prefix, user_id, permissions, last_used_at, expires_at, created_at, is_active
            FROM api_keys
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let permissions: Vec<String> =
                    serde_json::from_value(r.4).unwrap_or_default();

                ApiKey {
                    id: r.0,
                    name: r.1,
                    key_prefix: r.2,
                    user_id: r.3,
                    permissions,
                    last_used_at: r.5,
                    expires_at: r.6,
                    created_at: r.7,
                    is_active: r.8,
                }
            })
            .collect())
    }

    /// Revoke API key
    pub async fn revoke_api_key(&self, user_id: i32, key_id: i64) -> Result<()> {
        sqlx::query("UPDATE api_keys SET is_active = false WHERE id = $1 AND user_id = $2")
            .bind(key_id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Get user sessions
    pub async fn get_user_sessions(
        &self,
        user_id: i32,
        current_session_id: Option<&str>,
    ) -> Result<Vec<SessionInfo>> {
        let rows: Vec<SessionInfoRow> = sqlx::query_as(
            r#"
            SELECT id, user_id, ip_address, user_agent, device_type, location, created_at, last_active_at
            FROM user_sessions
            WHERE user_id = $1 AND is_active = true
            ORDER BY last_active_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SessionInfo {
                id: r.0.clone(),
                user_id: r.1,
                ip_address: r.2,
                user_agent: r.3,
                device_type: r.4,
                location: r.5,
                created_at: r.6,
                last_active_at: r.7,
                is_current: current_session_id.map(|s| s == r.0).unwrap_or(false),
            })
            .collect())
    }

    /// Revoke session
    pub async fn revoke_session(&self, user_id: i32, session_id: &str) -> Result<()> {
        sqlx::query("UPDATE user_sessions SET is_active = false WHERE id = $1 AND user_id = $2")
            .bind(session_id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Revoke all sessions except current
    pub async fn revoke_all_sessions(&self, user_id: i32, except_session_id: &str) -> Result<i64> {
        let result = sqlx::query(
            "UPDATE user_sessions SET is_active = false WHERE user_id = $1 AND id != $2",
        )
        .bind(user_id)
        .bind(except_session_id)
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Get security report
    pub async fn get_security_report(&self, days: i32) -> Result<SecurityReport> {
        let period_start = Utc::now() - Duration::days(days as i64);
        let period_end = Utc::now();

        let total_logins: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM security_events WHERE event_type = $1 AND created_at >= $2",
        )
        .bind(serde_json::to_string(&SecurityEventType::LoginSuccess)?)
        .bind(period_start)
        .fetch_one(&self.db)
        .await?;

        let failed_logins: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM security_events WHERE event_type = $1 AND created_at >= $2",
        )
        .bind(serde_json::to_string(&SecurityEventType::LoginFailed)?)
        .bind(period_start)
        .fetch_one(&self.db)
        .await?;

        let blocked_ips: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM ip_blocks WHERE blocked_at >= $1",
        )
        .bind(period_start)
        .fetch_one(&self.db)
        .await?;

        let rate_limit_hits: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM security_events WHERE event_type = $1 AND created_at >= $2",
        )
        .bind(serde_json::to_string(&SecurityEventType::RateLimitExceeded)?)
        .bind(period_start)
        .fetch_one(&self.db)
        .await?;

        let suspicious: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM security_events WHERE severity IN ($1, $2) AND created_at >= $3",
        )
        .bind(serde_json::to_string(&Severity::High)?)
        .bind(serde_json::to_string(&Severity::Critical)?)
        .bind(period_start)
        .fetch_one(&self.db)
        .await?;

        Ok(SecurityReport {
            period_start,
            period_end,
            total_logins: total_logins.0,
            failed_logins: failed_logins.0,
            blocked_ips: blocked_ips.0,
            rate_limit_hits: rate_limit_hits.0,
            suspicious_activities: suspicious.0,
            top_blocked_ips: vec![],
            events_by_type: vec![],
            events_by_severity: vec![],
        })
    }

    /// Get security settings
    pub async fn get_settings(&self) -> Result<SecuritySettings> {
        // TODO: Load from database
        Ok(SecuritySettings {
            require_mfa: false,
            session_timeout_minutes: 60,
            max_login_attempts: 5,
            lockout_duration_minutes: 30,
            password_min_length: 8,
            password_require_uppercase: true,
            password_require_lowercase: true,
            password_require_number: true,
            password_require_special: false,
            allowed_ip_ranges: vec![],
            blocked_ip_ranges: vec![],
        })
    }

    /// Update security settings
    pub async fn update_settings(&self, settings: SecuritySettings) -> Result<SecuritySettings> {
        // TODO: Save to database
        Ok(settings)
    }
}

/// Generate random API key
fn generate_api_key() -> String {
    let mut rng = rand::thread_rng();
    let key: String = (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..62);
            let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            chars[idx] as char
        })
        .collect();

    format!("mk_{}", key)
}

/// Hash API key for storage
fn hash_api_key(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Severity;

    #[test]
    fn test_generate_api_key_has_mk_prefix() {
        let key = generate_api_key();
        assert!(key.starts_with("mk_"), "API key should start with 'mk_'");
    }

    #[test]
    fn test_generate_api_key_length() {
        let key = generate_api_key();
        // "mk_" (3) + 32 alphanumeric chars = 35 total
        assert_eq!(key.len(), 35, "API key should be 35 characters (mk_ + 32)");
    }

    #[test]
    fn test_generate_api_key_alphanumeric_suffix() {
        let key = generate_api_key();
        let suffix = &key[3..]; // skip "mk_"
        assert!(
            suffix.chars().all(|c| c.is_ascii_alphanumeric()),
            "API key suffix should be alphanumeric only"
        );
    }

    #[test]
    fn test_generate_api_key_unique() {
        let key1 = generate_api_key();
        let key2 = generate_api_key();
        assert_ne!(key1, key2, "Each generated key should be unique");
    }

    #[test]
    fn test_hash_api_key_produces_hex_string() {
        let hash = hash_api_key("test_key");
        // SHA256 produces 64 hex characters
        assert_eq!(hash.len(), 64, "SHA256 hash should be 64 hex characters");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be hex");
    }

    #[test]
    fn test_hash_api_key_deterministic() {
        let hash1 = hash_api_key("same_key");
        let hash2 = hash_api_key("same_key");
        assert_eq!(hash1, hash2, "Same input should produce same hash");
    }

    #[test]
    fn test_hash_api_key_different_inputs_differ() {
        let hash1 = hash_api_key("key_one");
        let hash2 = hash_api_key("key_two");
        assert_ne!(hash1, hash2, "Different inputs should produce different hashes");
    }

    #[test]
    fn test_severity_ordering_info_is_lowest() {
        assert!(Severity::Info < Severity::Low);
        assert!(Severity::Info < Severity::Medium);
        assert!(Severity::Info < Severity::High);
        assert!(Severity::Info < Severity::Critical);
    }

    #[test]
    fn test_severity_ordering_critical_is_highest() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::Critical > Severity::Medium);
        assert!(Severity::Critical > Severity::Low);
        assert!(Severity::Critical > Severity::Info);
    }

    #[test]
    fn test_severity_ordering_preserves_sequence() {
        let mut levels = vec![
            Severity::Critical,
            Severity::Low,
            Severity::Info,
            Severity::High,
            Severity::Medium,
        ];
        levels.sort();
        assert_eq!(
            levels,
            vec![
                Severity::Info,
                Severity::Low,
                Severity::Medium,
                Severity::High,
                Severity::Critical,
            ]
        );
    }
}
