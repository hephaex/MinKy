use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{
        AnalyticsOverview, CategoryStats, ContentAnalysis, DashboardData, DocumentMetrics,
        TagStats, TimeSeriesPoint, UserMetrics, WorkflowAnalytics,
    },
    services::AnalyticsService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/overview", get(get_overview))
        .route("/dashboard", get(get_dashboard))
        .route("/documents/top", get(get_top_documents))
        .route("/documents/{id}/content", get(analyze_document_content))
        .route("/users/active", get(get_active_users))
        .route("/categories", get(get_category_stats))
        .route("/tags", get(get_tag_stats))
        .route("/timeline", get(get_activity_timeline))
        .route("/workflows", get(get_workflow_analytics))
}

#[derive(Debug, Deserialize)]
pub struct TimeRangeQuery {
    pub days: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct OverviewResponse {
    pub success: bool,
    pub data: AnalyticsOverview,
}

async fn get_overview(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<TimeRangeQuery>,
) -> AppResult<Json<OverviewResponse>> {
    let service = AnalyticsService::new(state.db.clone());
    let days = query.days.unwrap_or(30);
    let overview = service.get_overview(days).await?;

    Ok(Json(OverviewResponse {
        success: true,
        data: overview,
    }))
}

#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub success: bool,
    pub data: DashboardData,
}

async fn get_dashboard(State(state): State<AppState>, _auth_user: AuthUser) -> AppResult<Json<DashboardResponse>> {
    let service = AnalyticsService::new(state.db.clone());
    let dashboard = service.get_dashboard().await?;

    Ok(Json(DashboardResponse {
        success: true,
        data: dashboard,
    }))
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct DocumentsResponse {
    pub success: bool,
    pub data: Vec<DocumentMetrics>,
}

async fn get_top_documents(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<LimitQuery>,
) -> AppResult<Json<DocumentsResponse>> {
    let service = AnalyticsService::new(state.db.clone());
    let limit = query.limit.unwrap_or(10).min(100);
    let documents = service.get_top_documents(limit).await?;

    Ok(Json(DocumentsResponse {
        success: true,
        data: documents,
    }))
}

#[derive(Debug, Serialize)]
pub struct ContentAnalysisResponse {
    pub success: bool,
    pub data: ContentAnalysis,
}

async fn analyze_document_content(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(id): Path<uuid::Uuid>,
) -> AppResult<Json<ContentAnalysisResponse>> {
    // Fetch document content
    let content: (String,) = sqlx::query_as("SELECT content FROM documents WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    let analysis = AnalyticsService::analyze_content(&content.0);

    Ok(Json(ContentAnalysisResponse {
        success: true,
        data: analysis,
    }))
}

#[derive(Debug, Serialize)]
pub struct UsersResponse {
    pub success: bool,
    pub data: Vec<UserMetrics>,
}

async fn get_active_users(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<LimitQuery>,
) -> AppResult<Json<UsersResponse>> {
    let service = AnalyticsService::new(state.db.clone());
    let limit = query.limit.unwrap_or(10).min(100);
    let users = service.get_active_users(limit).await?;

    Ok(Json(UsersResponse {
        success: true,
        data: users,
    }))
}

#[derive(Debug, Serialize)]
pub struct CategoriesResponse {
    pub success: bool,
    pub data: Vec<CategoryStats>,
}

async fn get_category_stats(State(state): State<AppState>, _auth_user: AuthUser) -> AppResult<Json<CategoriesResponse>> {
    let service = AnalyticsService::new(state.db.clone());
    let categories = service.get_category_stats().await?;

    Ok(Json(CategoriesResponse {
        success: true,
        data: categories,
    }))
}

#[derive(Debug, Serialize)]
pub struct TagsResponse {
    pub success: bool,
    pub data: Vec<TagStats>,
}

async fn get_tag_stats(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<LimitQuery>,
) -> AppResult<Json<TagsResponse>> {
    let service = AnalyticsService::new(state.db.clone());
    let limit = query.limit.unwrap_or(50).min(200);
    let tags = service.get_tag_stats(limit).await?;

    Ok(Json(TagsResponse {
        success: true,
        data: tags,
    }))
}

#[derive(Debug, Serialize)]
pub struct TimelineResponse {
    pub success: bool,
    pub data: Vec<TimeSeriesPoint>,
}

async fn get_activity_timeline(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<TimeRangeQuery>,
) -> AppResult<Json<TimelineResponse>> {
    let service = AnalyticsService::new(state.db.clone());
    let days = query.days.unwrap_or(30).min(365);
    let timeline = service.get_activity_timeline(days).await?;

    Ok(Json(TimelineResponse {
        success: true,
        data: timeline,
    }))
}

#[derive(Debug, Serialize)]
pub struct WorkflowAnalyticsResponse {
    pub success: bool,
    pub data: WorkflowAnalytics,
}

async fn get_workflow_analytics(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> AppResult<Json<WorkflowAnalyticsResponse>> {
    let service = AnalyticsService::new(state.db.clone());
    let analytics = service.get_workflow_analytics().await?;

    Ok(Json(WorkflowAnalyticsResponse {
        success: true,
        data: analytics,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // TimeRangeQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_time_range_query_deserialization() {
        let json = r#"{"days": 7}"#;
        let query: TimeRangeQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.days, Some(7));
    }

    #[test]
    fn test_time_range_query_default() {
        let json = r#"{}"#;
        let query: TimeRangeQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.days, None);
    }

    #[test]
    fn test_time_range_default_logic() {
        // Test the default logic used in handlers
        let test_cases = vec![
            (None, 30),   // default
            (Some(7), 7), // specified
            (Some(365), 365),
        ];

        for (input, expected) in test_cases {
            let days = input.unwrap_or(30);
            assert_eq!(days, expected);
        }
    }

    // -------------------------------------------------------------------------
    // LimitQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_limit_query_deserialization() {
        let json = r#"{"limit": 25}"#;
        let query: LimitQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, Some(25));
    }

    #[test]
    fn test_limit_query_default() {
        let json = r#"{}"#;
        let query: LimitQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, None);
    }

    #[test]
    fn test_limit_clamping_logic() {
        // Test limit clamping used in get_top_documents
        let test_cases = vec![
            (None, 10),     // default
            (Some(50), 50), // below max
            (Some(100), 100), // at max
            (Some(200), 100), // above max, clamped
        ];

        for (input, expected) in test_cases {
            let limit = input.unwrap_or(10).min(100);
            assert_eq!(limit, expected, "Failed for input {:?}", input);
        }
    }

    #[test]
    fn test_tags_limit_clamping() {
        // Test limit clamping used in get_tag_stats (max 200)
        let test_cases = vec![
            (None, 50),     // default
            (Some(100), 100),
            (Some(200), 200), // at max
            (Some(300), 200), // above max, clamped
        ];

        for (input, expected) in test_cases {
            let limit = input.unwrap_or(50).min(200);
            assert_eq!(limit, expected, "Failed for input {:?}", input);
        }
    }

    // -------------------------------------------------------------------------
    // Response tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_overview_response_serialization() {
        let response = OverviewResponse {
            success: true,
            data: AnalyticsOverview {
                total_documents: 100,
                total_views: 500,
                total_comments: 50,
                total_users: 10,
                documents_this_period: 15,
                views_this_period: 100,
                active_users: 7,
                avg_document_length: 1500.5,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"total_documents\":100"));
    }

    #[test]
    fn test_documents_response_serialization() {
        let response = DocumentsResponse {
            success: true,
            data: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":[]"));
    }

    #[test]
    fn test_users_response_serialization() {
        let response = UsersResponse {
            success: true,
            data: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":[]"));
    }

    #[test]
    fn test_categories_response_serialization() {
        let response = CategoriesResponse {
            success: true,
            data: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_tags_response_serialization() {
        let response = TagsResponse {
            success: true,
            data: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_timeline_response_serialization() {
        let response = TimelineResponse {
            success: true,
            data: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
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
