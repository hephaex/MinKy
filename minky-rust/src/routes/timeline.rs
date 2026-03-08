use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::Deserialize;

use crate::middleware::AuthUser;
use crate::models::{
    ActivityHeatmap, DailyActivity, DocumentHistory, TimelineEventType, TimelineQuery,
    TimelineResponse, TimelineStats, UserActivityStream,
};
use crate::services::TimelineService;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct DailyActivityQuery {
    pub days: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct HeatmapQuery {
    pub year: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UserActivityQuery {
    pub limit: Option<i32>,
}

/// Get timeline events
async fn get_timeline(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<TimelineResponse>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());

    service
        .get_timeline(query)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Log timeline event
async fn log_event(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(request): Json<LogEventRequest>,
) -> Result<Json<LogEventResponse>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());

    service
        .log_event(
            request.event_type,
            request.user_id,
            request.document_id,
            &request.description,
            request.metadata,
        )
        .await
        .map(|event_id| Json(LogEventResponse { event_id }))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get daily activity
async fn get_daily_activity(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<DailyActivityQuery>,
) -> Result<Json<Vec<DailyActivity>>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());
    let days = query.days.unwrap_or(30);

    service
        .get_daily_activity(days)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get activity heatmap
async fn get_heatmap(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<HeatmapQuery>,
) -> Result<Json<ActivityHeatmap>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());
    let year = query.year.unwrap_or(chrono::Utc::now().year());

    service
        .get_activity_heatmap(year)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get user activity stream
async fn get_user_activity(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(user_id): Path<i32>,
    Query(query): Query<UserActivityQuery>,
) -> Result<Json<UserActivityStream>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());
    let limit = query.limit.unwrap_or(50);

    service
        .get_user_activity(user_id, limit)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get document history
async fn get_document_history(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<DocumentHistory>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());

    service
        .get_document_history(document_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get timeline statistics
async fn get_stats(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> Result<Json<TimelineStats>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());

    service
        .get_stats()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct LogEventRequest {
    pub event_type: TimelineEventType,
    pub user_id: i32,
    pub document_id: Option<uuid::Uuid>,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, serde::Serialize)]
pub struct LogEventResponse {
    pub event_id: String,
}

use chrono::Datelike;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_timeline).post(log_event))
        .route("/daily", get(get_daily_activity))
        .route("/heatmap", get(get_heatmap))
        .route("/users/{user_id}", get(get_user_activity))
        .route("/documents/{document_id}", get(get_document_history))
        .route("/stats", get(get_stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ActiveDocumentInfo, ActiveUserInfo, ContributorInfo, EventTypeCount, HeatmapCell,
        TimelineEvent,
    };
    use chrono::NaiveDate;
    use uuid::Uuid;

    // -------------------------------------------------------------------------
    // DailyActivityQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_daily_activity_query_deserialization() {
        let json = r#"{"days": 14}"#;
        let query: DailyActivityQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.days, Some(14));
    }

    #[test]
    fn test_daily_activity_query_default() {
        let json = r#"{}"#;
        let query: DailyActivityQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.days, None);
    }

    #[test]
    fn test_daily_activity_default_logic() {
        let test_cases = vec![
            (None, 30),   // default
            (Some(7), 7), // specified
            (Some(90), 90),
        ];

        for (input, expected) in test_cases {
            let days = input.unwrap_or(30);
            assert_eq!(days, expected);
        }
    }

    // -------------------------------------------------------------------------
    // HeatmapQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_heatmap_query_deserialization() {
        let json = r#"{"year": 2026}"#;
        let query: HeatmapQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.year, Some(2026));
    }

    #[test]
    fn test_heatmap_query_default() {
        let json = r#"{}"#;
        let query: HeatmapQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.year, None);
    }

    // -------------------------------------------------------------------------
    // UserActivityQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_user_activity_query_deserialization() {
        let json = r#"{"limit": 100}"#;
        let query: UserActivityQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, Some(100));
    }

    #[test]
    fn test_user_activity_query_default_logic() {
        let test_cases = vec![
            (None, 50),      // default
            (Some(25), 25),  // specified
            (Some(100), 100),
        ];

        for (input, expected) in test_cases {
            let limit = input.unwrap_or(50);
            assert_eq!(limit, expected);
        }
    }

    // -------------------------------------------------------------------------
    // LogEventRequest tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_log_event_request_deserialization() {
        let doc_id = Uuid::new_v4();
        let json = format!(
            r#"{{"event_type": "document_created", "user_id": 1, "document_id": "{}", "description": "Created new doc"}}"#,
            doc_id
        );
        let request: LogEventRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.user_id, 1);
        assert_eq!(request.document_id, Some(doc_id));
        assert_eq!(request.description, "Created new doc");
    }

    #[test]
    fn test_log_event_request_without_document() {
        let json = r#"{"event_type": "comment_added", "user_id": 5, "description": "Comment added"}"#;
        let request: LogEventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.user_id, 5);
        assert!(request.document_id.is_none());
    }

    #[test]
    fn test_log_event_request_with_metadata() {
        let json = r#"{"event_type": "document_updated", "user_id": 1, "description": "Updated", "metadata": {"field": "title"}}"#;
        let request: LogEventRequest = serde_json::from_str(json).unwrap();
        assert!(request.metadata.is_some());
    }

    // -------------------------------------------------------------------------
    // LogEventResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_log_event_response_serialization() {
        let response = LogEventResponse {
            event_id: "evt-12345".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"event_id\":\"evt-12345\""));
    }

    // -------------------------------------------------------------------------
    // TimelineResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_timeline_response_serialization() {
        let response = TimelineResponse {
            events: vec![],
            total: 0,
            page: 1,
            limit: 50,
            has_more: false,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"has_more\":false"));
    }

    #[test]
    fn test_timeline_response_with_events() {
        let now = chrono::Utc::now();
        let response = TimelineResponse {
            events: vec![TimelineEvent {
                id: "evt-001".to_string(),
                event_type: TimelineEventType::DocumentCreated,
                document_id: Some(Uuid::new_v4()),
                document_title: Some("Test Doc".to_string()),
                user_id: 1,
                username: "testuser".to_string(),
                description: "Created document".to_string(),
                metadata: None,
                timestamp: now,
            }],
            total: 1,
            page: 1,
            limit: 50,
            has_more: false,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("\"username\":\"testuser\""));
    }

    // -------------------------------------------------------------------------
    // DailyActivity tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_daily_activity_serialization() {
        let activity = DailyActivity {
            date: NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(),
            total_events: 100,
            documents_created: 10,
            documents_updated: 20,
            comments_added: 30,
            active_users: 15,
        };
        let json = serde_json::to_string(&activity).unwrap();
        assert!(json.contains("\"total_events\":100"));
        assert!(json.contains("\"active_users\":15"));
    }

    // -------------------------------------------------------------------------
    // ActivityHeatmap tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_activity_heatmap_serialization() {
        let heatmap = ActivityHeatmap {
            data: vec![HeatmapCell {
                date: NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(),
                value: 10,
                level: 2,
            }],
            start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            max_value: 50,
        };
        let json = serde_json::to_string(&heatmap).unwrap();
        assert!(json.contains("\"max_value\":50"));
        assert!(json.contains("\"level\":2"));
    }

    // -------------------------------------------------------------------------
    // UserActivityStream tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_user_activity_stream_serialization() {
        let stream = UserActivityStream {
            user_id: 1,
            username: "testuser".to_string(),
            events: vec![],
            total_events: 0,
            most_active_day: Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
            streak_days: 7,
        };
        let json = serde_json::to_string(&stream).unwrap();
        assert!(json.contains("\"username\":\"testuser\""));
        assert!(json.contains("\"streak_days\":7"));
    }

    // -------------------------------------------------------------------------
    // DocumentHistory tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_document_history_serialization() {
        let now = chrono::Utc::now();
        let history = DocumentHistory {
            document_id: Uuid::new_v4(),
            title: "My Document".to_string(),
            events: vec![],
            total_views: 100,
            total_edits: 25,
            contributors: vec![ContributorInfo {
                user_id: 1,
                username: "contributor".to_string(),
                contribution_count: 10,
                last_contribution: now,
            }],
        };
        let json = serde_json::to_string(&history).unwrap();
        assert!(json.contains("\"title\":\"My Document\""));
        assert!(json.contains("\"total_views\":100"));
        assert!(json.contains("\"contribution_count\":10"));
    }

    // -------------------------------------------------------------------------
    // TimelineStats tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_timeline_stats_serialization() {
        let stats = TimelineStats {
            total_events: 1000,
            events_today: 50,
            events_this_week: 200,
            events_this_month: 500,
            by_type: vec![EventTypeCount {
                event_type: TimelineEventType::DocumentCreated,
                count: 100,
            }],
            most_active_users: vec![ActiveUserInfo {
                user_id: 1,
                username: "topuser".to_string(),
                event_count: 500,
            }],
            most_active_documents: vec![ActiveDocumentInfo {
                document_id: Uuid::new_v4(),
                title: "Popular Doc".to_string(),
                event_count: 200,
            }],
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_events\":1000"));
        assert!(json.contains("\"events_today\":50"));
        assert!(json.contains("\"username\":\"topuser\""));
    }

    // -------------------------------------------------------------------------
    // Router tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_router_creation() {
        let _router: Router<AppState> = router();
        // Should be creatable without panicking
    }
}
