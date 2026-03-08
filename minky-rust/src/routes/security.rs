use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{
        ApiKey, ApiKeyWithSecret, BlockIpRequest, CreateApiKeyRequest, IpBlock, SecurityEvent,
        SecurityEventType, SecurityReport, SecuritySettings, SessionInfo, Severity,
    },
    services::SecurityService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/events", get(get_security_events))
        .route("/ip-blocks", get(list_blocked_ips))
        .route("/ip-blocks", post(block_ip))
        .route("/ip-blocks/{ip}", delete(unblock_ip))
        .route("/api-keys", get(list_api_keys))
        .route("/api-keys", post(create_api_key))
        .route("/api-keys/{id}", delete(revoke_api_key))
        .route("/sessions", get(get_sessions))
        .route("/sessions/{id}", delete(revoke_session))
        .route("/sessions/revoke-all", post(revoke_all_sessions))
        .route("/report", get(get_security_report))
        .route("/settings", get(get_settings))
        .route("/settings", put(update_settings))
}

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub severity: Option<Severity>,
    pub event_type: Option<SecurityEventType>,
}

#[derive(Debug, Serialize)]
pub struct EventsResponse {
    pub success: bool,
    pub data: Vec<SecurityEvent>,
}

async fn get_security_events(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<EventsQuery>,
) -> AppResult<Json<EventsResponse>> {
    let service = SecurityService::new(state.db.clone());
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(50).min(200);

    let events = service
        .get_events(page, limit, query.severity, query.event_type)
        .await?;

    Ok(Json(EventsResponse {
        success: true,
        data: events,
    }))
}

#[derive(Debug, Serialize)]
pub struct IpBlocksResponse {
    pub success: bool,
    pub data: Vec<IpBlock>,
}

async fn list_blocked_ips(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> AppResult<Json<IpBlocksResponse>> {
    let service = SecurityService::new(state.db.clone());
    let blocks = service.get_blocked_ips().await?;

    Ok(Json(IpBlocksResponse {
        success: true,
        data: blocks,
    }))
}

#[derive(Debug, Serialize)]
pub struct IpBlockResponse {
    pub success: bool,
    pub data: IpBlock,
}

async fn block_ip(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<BlockIpRequest>,
) -> AppResult<Json<IpBlockResponse>> {
    let service = SecurityService::new(state.db.clone());
    let block = service.block_ip(auth_user.id, payload).await?;

    Ok(Json(IpBlockResponse {
        success: true,
        data: block,
    }))
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub success: bool,
    pub message: String,
}

async fn unblock_ip(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(ip): Path<String>,
) -> AppResult<Json<MessageResponse>> {
    let service = SecurityService::new(state.db.clone());
    service.unblock_ip(&ip).await?;

    Ok(Json(MessageResponse {
        success: true,
        message: "IP unblocked".to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub struct ApiKeysResponse {
    pub success: bool,
    pub data: Vec<ApiKey>,
}

async fn list_api_keys(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<ApiKeysResponse>> {
    let service = SecurityService::new(state.db.clone());
    let keys = service.list_api_keys(auth_user.id).await?;

    Ok(Json(ApiKeysResponse {
        success: true,
        data: keys,
    }))
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub success: bool,
    pub data: ApiKeyWithSecret,
}

async fn create_api_key(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateApiKeyRequest>,
) -> AppResult<Json<ApiKeyResponse>> {
    let service = SecurityService::new(state.db.clone());
    let key = service.create_api_key(auth_user.id, payload).await?;

    Ok(Json(ApiKeyResponse {
        success: true,
        data: key,
    }))
}

async fn revoke_api_key(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<MessageResponse>> {
    let service = SecurityService::new(state.db.clone());
    service.revoke_api_key(auth_user.id, id).await?;

    Ok(Json(MessageResponse {
        success: true,
        message: "API key revoked".to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub struct SessionsResponse {
    pub success: bool,
    pub data: Vec<SessionInfo>,
}

async fn get_sessions(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<SessionsResponse>> {
    let service = SecurityService::new(state.db.clone());
    let sessions = service
        .get_user_sessions(auth_user.id, None)
        .await?;

    Ok(Json(SessionsResponse {
        success: true,
        data: sessions,
    }))
}

async fn revoke_session(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<MessageResponse>> {
    let service = SecurityService::new(state.db.clone());
    service.revoke_session(auth_user.id, &id).await?;

    Ok(Json(MessageResponse {
        success: true,
        message: "Session revoked".to_string(),
    }))
}

async fn revoke_all_sessions(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<MessageResponse>> {
    // Revoke all sessions for the user (session tracking is handled by service layer)
    let service = SecurityService::new(state.db.clone());
    let count = service
        .revoke_all_sessions(auth_user.id, "current")
        .await?;

    Ok(Json(MessageResponse {
        success: true,
        message: format!("Revoked {} session(s)", count),
    }))
}

#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    pub days: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ReportResponse {
    pub success: bool,
    pub data: SecurityReport,
}

async fn get_security_report(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<ReportQuery>,
) -> AppResult<Json<ReportResponse>> {
    let service = SecurityService::new(state.db.clone());
    let days = query.days.unwrap_or(30).min(365);
    let report = service.get_security_report(days).await?;

    Ok(Json(ReportResponse {
        success: true,
        data: report,
    }))
}

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub success: bool,
    pub data: SecuritySettings,
}

async fn get_settings(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> AppResult<Json<SettingsResponse>> {
    let service = SecurityService::new(state.db.clone());
    let settings = service.get_settings().await?;

    Ok(Json(SettingsResponse {
        success: true,
        data: settings,
    }))
}

async fn update_settings(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<SecuritySettings>,
) -> AppResult<Json<SettingsResponse>> {
    let service = SecurityService::new(state.db.clone());
    let settings = service.update_settings(payload).await?;

    Ok(Json(SettingsResponse {
        success: true,
        data: settings,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SecurityReport;

    // -------------------------------------------------------------------------
    // EventsQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_events_query_deserialization() {
        let json = r#"{"page": 2, "limit": 50, "severity": "high"}"#;
        let query: EventsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page, Some(2));
        assert_eq!(query.limit, Some(50));
        assert!(matches!(query.severity, Some(Severity::High)));
    }

    #[test]
    fn test_events_query_with_event_type() {
        let json = r#"{"event_type": "login_success"}"#;
        let query: EventsQuery = serde_json::from_str(json).unwrap();
        assert!(matches!(query.event_type, Some(SecurityEventType::LoginSuccess)));
    }

    #[test]
    fn test_events_query_empty() {
        let json = r#"{}"#;
        let query: EventsQuery = serde_json::from_str(json).unwrap();
        assert!(query.page.is_none());
        assert!(query.limit.is_none());
        assert!(query.severity.is_none());
        assert!(query.event_type.is_none());
    }

    // -------------------------------------------------------------------------
    // EventsResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_events_response_serialization() {
        let now = chrono::Utc::now();
        let response = EventsResponse {
            success: true,
            data: vec![SecurityEvent {
                id: 1,
                event_type: SecurityEventType::LoginSuccess,
                severity: Severity::Info,
                user_id: Some(1),
                username: Some("admin".to_string()),
                ip_address: Some("192.168.1.1".to_string()),
                user_agent: None,
                resource_type: None,
                resource_id: None,
                details: None,
                created_at: now,
            }],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"event_type\":\"login_success\""));
    }

    #[test]
    fn test_events_response_empty() {
        let response = EventsResponse {
            success: true,
            data: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"data\":[]"));
    }

    // -------------------------------------------------------------------------
    // IpBlocksResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_ip_blocks_response_serialization() {
        let now = chrono::Utc::now();
        let response = IpBlocksResponse {
            success: true,
            data: vec![IpBlock {
                id: 1,
                ip_address: "10.0.0.1".to_string(),
                reason: "Suspicious activity".to_string(),
                blocked_by: Some(1),
                blocked_at: now,
                expires_at: None,
                is_permanent: true,
            }],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"ip_address\":\"10.0.0.1\""));
        assert!(json.contains("\"is_permanent\":true"));
    }

    // -------------------------------------------------------------------------
    // IpBlockResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_ip_block_response_serialization() {
        let now = chrono::Utc::now();
        let response = IpBlockResponse {
            success: true,
            data: IpBlock {
                id: 2,
                ip_address: "192.168.1.100".to_string(),
                reason: "Brute force attack".to_string(),
                blocked_by: Some(1),
                blocked_at: now,
                expires_at: Some(now + chrono::Duration::hours(24)),
                is_permanent: false,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"reason\":\"Brute force attack\""));
        assert!(json.contains("\"is_permanent\":false"));
    }

    // -------------------------------------------------------------------------
    // MessageResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_message_response_serialization() {
        let response = MessageResponse {
            success: true,
            message: "Operation completed".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"Operation completed\""));
    }

    // -------------------------------------------------------------------------
    // ApiKeysResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_api_keys_response_serialization() {
        let now = chrono::Utc::now();
        let response = ApiKeysResponse {
            success: true,
            data: vec![ApiKey {
                id: 1,
                name: "Production Key".to_string(),
                key_prefix: "sk_prod_".to_string(),
                user_id: 1,
                permissions: vec!["read".to_string(), "write".to_string()],
                last_used_at: Some(now),
                expires_at: None,
                created_at: now,
                is_active: true,
            }],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"Production Key\""));
        assert!(json.contains("\"is_active\":true"));
    }

    // -------------------------------------------------------------------------
    // ApiKeyResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_api_key_response_serialization() {
        let now = chrono::Utc::now();
        let response = ApiKeyResponse {
            success: true,
            data: ApiKeyWithSecret {
                id: 1,
                name: "New Key".to_string(),
                key: "sk_test_abc123xyz".to_string(),
                permissions: vec!["read".to_string()],
                expires_at: Some(now + chrono::Duration::days(30)),
                created_at: now,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"key\":\"sk_test_abc123xyz\""));
    }

    // -------------------------------------------------------------------------
    // SessionsResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_sessions_response_serialization() {
        let now = chrono::Utc::now();
        let response = SessionsResponse {
            success: true,
            data: vec![SessionInfo {
                id: "sess_abc123".to_string(),
                user_id: 1,
                ip_address: "192.168.1.50".to_string(),
                user_agent: "Mozilla/5.0".to_string(),
                device_type: Some("desktop".to_string()),
                location: Some("Seoul, KR".to_string()),
                created_at: now,
                last_active_at: now,
                is_current: true,
            }],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"sess_abc123\""));
        assert!(json.contains("\"is_current\":true"));
    }

    // -------------------------------------------------------------------------
    // ReportQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_report_query_deserialization() {
        let json = r#"{"days": 7}"#;
        let query: ReportQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.days, Some(7));
    }

    #[test]
    fn test_report_query_empty() {
        let json = r#"{}"#;
        let query: ReportQuery = serde_json::from_str(json).unwrap();
        assert!(query.days.is_none());
    }

    // -------------------------------------------------------------------------
    // ReportResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_report_response_serialization() {
        let now = chrono::Utc::now();
        let response = ReportResponse {
            success: true,
            data: SecurityReport {
                period_start: now - chrono::Duration::days(30),
                period_end: now,
                total_logins: 500,
                failed_logins: 25,
                blocked_ips: 3,
                rate_limit_hits: 10,
                suspicious_activities: 2,
                top_blocked_ips: vec![("10.0.0.1".to_string(), 5)],
                events_by_type: vec![("login_success".to_string(), 475)],
                events_by_severity: vec![("info".to_string(), 480)],
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total_logins\":500"));
        assert!(json.contains("\"failed_logins\":25"));
    }

    // -------------------------------------------------------------------------
    // SettingsResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_settings_response_serialization() {
        let response = SettingsResponse {
            success: true,
            data: SecuritySettings {
                require_mfa: true,
                session_timeout_minutes: 60,
                max_login_attempts: 5,
                lockout_duration_minutes: 30,
                password_min_length: 12,
                password_require_uppercase: true,
                password_require_lowercase: true,
                password_require_number: true,
                password_require_special: true,
                allowed_ip_ranges: vec!["10.0.0.0/8".to_string()],
                blocked_ip_ranges: vec![],
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"require_mfa\":true"));
        assert!(json.contains("\"password_min_length\":12"));
    }

    // -------------------------------------------------------------------------
    // Router tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_routes_creation() {
        let _router: Router<AppState> = routes();
        // Should be creatable without panicking
    }
}
