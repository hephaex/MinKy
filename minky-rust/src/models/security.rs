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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering_info_is_lowest() {
        assert!(Severity::Info < Severity::Low);
        assert!(Severity::Info < Severity::Medium);
        assert!(Severity::Info < Severity::Critical);
    }

    #[test]
    fn test_severity_ordering_critical_is_highest() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::Critical > Severity::Medium);
        assert!(Severity::Critical > Severity::Low);
    }

    #[test]
    fn test_severity_ordering_full_chain() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
    }

    #[test]
    fn test_security_event_structure() {
        let event = SecurityEvent {
            id: 1,
            event_type: SecurityEventType::LoginSuccess,
            severity: Severity::Info,
            user_id: Some(42),
            username: Some("testuser".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: None,
            resource_type: None,
            resource_id: None,
            details: None,
            created_at: Utc::now(),
        };

        assert_eq!(event.id, 1);
        assert_eq!(event.user_id, Some(42));
        assert_eq!(event.severity, Severity::Info);
    }

    #[test]
    fn test_ip_block_structure() {
        let block = IpBlock {
            id: 1,
            ip_address: "203.0.113.0".to_string(),
            reason: "Brute force attempt".to_string(),
            blocked_by: Some(1),
            blocked_at: Utc::now(),
            expires_at: None,
            is_permanent: true,
        };

        assert_eq!(block.ip_address, "203.0.113.0");
        assert!(block.is_permanent);
    }

    #[test]
    fn test_rate_limit_status() {
        let status = RateLimitStatus {
            endpoint: "/api/search".to_string(),
            limit: 100,
            remaining: 45,
            reset_at: Utc::now(),
            retry_after_seconds: None,
        };

        assert_eq!(status.limit, 100);
        assert_eq!(status.remaining, 45);
        assert!(status.remaining < status.limit);
    }

    #[test]
    fn test_api_key_structure() {
        let key = ApiKey {
            id: 1,
            name: "Test Key".to_string(),
            key_prefix: "sk_test".to_string(),
            user_id: 1,
            permissions: vec!["read:documents".to_string()],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            is_active: true,
        };

        assert_eq!(key.name, "Test Key");
        assert!(key.is_active);
        assert_eq!(key.permissions.len(), 1);
    }

    #[test]
    fn test_session_info_structure() {
        let session = SessionInfo {
            id: "sess_123".to_string(),
            user_id: 5,
            ip_address: "192.168.1.50".to_string(),
            user_agent: "Mozilla/5.0".to_string(),
            device_type: Some("desktop".to_string()),
            location: Some("New York".to_string()),
            created_at: Utc::now(),
            last_active_at: Utc::now(),
            is_current: true,
        };

        assert_eq!(session.user_id, 5);
        assert!(session.is_current);
    }

    #[test]
    fn test_security_settings_structure() {
        let settings = SecuritySettings {
            require_mfa: true,
            session_timeout_minutes: 30,
            max_login_attempts: 5,
            lockout_duration_minutes: 15,
            password_min_length: 12,
            password_require_uppercase: true,
            password_require_lowercase: true,
            password_require_number: true,
            password_require_special: true,
            allowed_ip_ranges: vec!["10.0.0.0/8".to_string()],
            blocked_ip_ranges: vec![],
        };

        assert!(settings.require_mfa);
        assert_eq!(settings.password_min_length, 12);
    }

    #[test]
    fn test_login_attempt_success() {
        let attempt = LoginAttempt {
            id: 1,
            username: "user1".to_string(),
            ip_address: "192.168.1.100".to_string(),
            user_agent: "Chrome".to_string(),
            success: true,
            failure_reason: None,
            created_at: Utc::now(),
        };

        assert!(attempt.success);
        assert!(attempt.failure_reason.is_none());
    }

    #[test]
    fn test_login_attempt_failure() {
        let attempt = LoginAttempt {
            id: 2,
            username: "user2".to_string(),
            ip_address: "192.168.1.101".to_string(),
            user_agent: "Firefox".to_string(),
            success: false,
            failure_reason: Some("Invalid password".to_string()),
            created_at: Utc::now(),
        };

        assert!(!attempt.success);
        assert!(attempt.failure_reason.is_some());
    }

    #[test]
    fn test_create_api_key_request() {
        let req = CreateApiKeyRequest {
            name: "My API Key".to_string(),
            permissions: Some(vec!["read".to_string(), "write".to_string()]),
            expires_in_days: Some(30),
        };

        assert_eq!(req.name, "My API Key");
        assert_eq!(req.permissions.unwrap().len(), 2);
    }

    #[test]
    fn test_block_ip_request() {
        let req = BlockIpRequest {
            ip_address: "192.0.2.1".to_string(),
            reason: "Suspicious activity".to_string(),
            duration_hours: Some(24),
            is_permanent: Some(false),
        };

        assert_eq!(req.ip_address, "192.0.2.1");
        assert_eq!(req.duration_hours, Some(24));
    }

    #[test]
    fn test_security_event_types_exist() {
        let _login_success = SecurityEventType::LoginSuccess;
        let _login_failed = SecurityEventType::LoginFailed;
        let _mfa_enabled = SecurityEventType::MfaEnabled;
        let _rate_limit = SecurityEventType::RateLimitExceeded;

        // Just verify they exist and can be created
        assert!(matches!(_login_success, SecurityEventType::LoginSuccess));
    }

    #[test]
    fn test_severity_equality() {
        assert_eq!(Severity::Info, Severity::Info);
        assert_ne!(Severity::Info, Severity::Critical);
    }

    #[test]
    fn test_security_report_structure() {
        let report = SecurityReport {
            period_start: Utc::now(),
            period_end: Utc::now(),
            total_logins: 1000,
            failed_logins: 50,
            blocked_ips: 5,
            rate_limit_hits: 20,
            suspicious_activities: 3,
            top_blocked_ips: vec![("192.0.2.1".to_string(), 10)],
            events_by_type: vec![("login".to_string(), 500)],
            events_by_severity: vec![("info".to_string(), 950)],
        };

        assert_eq!(report.total_logins, 1000);
        assert_eq!(report.blocked_ips, 5);
    }
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
