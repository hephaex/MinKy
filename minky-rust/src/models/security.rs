use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Security event
#[derive(Debug, Serialize)]
pub struct SecurityEvent {
    pub id: i64,
    pub event_type: SecurityEventType,
    pub severity: Severity,
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    LoginSuccess,
    LoginFailed,
    LoginBlocked,
    LogoutSuccess,
    PasswordChanged,
    PasswordResetRequested,
    MfaEnabled,
    MfaDisabled,
    MfaFailed,
    SessionExpired,
    SessionRevoked,
    ApiKeyCreated,
    ApiKeyRevoked,
    ApiKeyUsed,
    PermissionDenied,
    RateLimitExceeded,
    SuspiciousActivity,
    BruteForceDetected,
    IpBlocked,
    IpUnblocked,
}

/// Severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// IP block entry
#[derive(Debug, Serialize)]
pub struct IpBlock {
    pub id: i64,
    pub ip_address: String,
    pub reason: String,
    pub blocked_by: Option<i32>,
    pub blocked_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_permanent: bool,
}

/// Block IP request
#[derive(Debug, Deserialize)]
pub struct BlockIpRequest {
    pub ip_address: String,
    pub reason: String,
    pub duration_hours: Option<i32>,
    pub is_permanent: Option<bool>,
}

/// Rate limit status
#[derive(Debug, Serialize)]
pub struct RateLimitStatus {
    pub endpoint: String,
    pub limit: i32,
    pub remaining: i32,
    pub reset_at: DateTime<Utc>,
    pub retry_after_seconds: Option<i32>,
}

/// API key
#[derive(Debug, Serialize)]
pub struct ApiKey {
    pub id: i64,
    pub name: String,
    pub key_prefix: String,
    pub user_id: i32,
    pub permissions: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

/// Create API key request
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub permissions: Option<Vec<String>>,
    pub expires_in_days: Option<i32>,
}

/// API key with secret (only shown once)
#[derive(Debug, Serialize)]
pub struct ApiKeyWithSecret {
    pub id: i64,
    pub name: String,
    pub key: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Session info
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub user_id: i32,
    pub ip_address: String,
    pub user_agent: String,
    pub device_type: Option<String>,
    pub location: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub is_current: bool,
}

/// Security settings
#[derive(Debug, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub require_mfa: bool,
    pub session_timeout_minutes: i32,
    pub max_login_attempts: i32,
    pub lockout_duration_minutes: i32,
    pub password_min_length: i32,
    pub password_require_uppercase: bool,
    pub password_require_lowercase: bool,
    pub password_require_number: bool,
    pub password_require_special: bool,
    pub allowed_ip_ranges: Vec<String>,
    pub blocked_ip_ranges: Vec<String>,
}

/// Login attempt
#[derive(Debug, Serialize)]
pub struct LoginAttempt {
    pub id: i64,
    pub username: String,
    pub ip_address: String,
    pub user_agent: String,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Security report
#[derive(Debug, Serialize)]
pub struct SecurityReport {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_logins: i64,
    pub failed_logins: i64,
    pub blocked_ips: i64,
    pub rate_limit_hits: i64,
    pub suspicious_activities: i64,
    pub top_blocked_ips: Vec<(String, i64)>,
    pub events_by_type: Vec<(String, i64)>,
    pub events_by_severity: Vec<(String, i64)>,
}
