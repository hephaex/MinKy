use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
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

async fn get_dashboard(State(state): State<AppState>) -> AppResult<Json<DashboardResponse>> {
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

async fn get_category_stats(State(state): State<AppState>) -> AppResult<Json<CategoriesResponse>> {
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
) -> AppResult<Json<WorkflowAnalyticsResponse>> {
    let service = AnalyticsService::new(state.db.clone());
    let analytics = service.get_workflow_analytics().await?;

    Ok(Json(WorkflowAnalyticsResponse {
        success: true,
        data: analytics,
    }))
}
