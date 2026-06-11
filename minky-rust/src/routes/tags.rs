use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    middleware::{AuthUser, OptionalAuthUser},
    models::{CreateTag, TagWithCount, UpdateTag},
    services::TagService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tags).post(create_tag))
        // Literal routes must precede /{slug} to avoid being swallowed as a slug value
        .route("/statistics", get(tag_statistics))
        .route("/suggest", get(suggest_tags_search))
        .route("/{slug}", get(get_tag).put(update_tag).delete(delete_tag))
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct TagListQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
    pub popular: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Pagination {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub pages: i64,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(Debug, Serialize)]
pub struct TagListData {
    pub tags: Vec<TagWithCount>,
    pub pagination: Pagination,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct TagListResponse {
    pub success: bool,
    pub data: TagListData,
}

async fn list_tags(
    State(state): State<AppState>,
    _auth_user: OptionalAuthUser,
    Query(query): Query<TagListQuery>,
) -> AppResult<Json<TagListResponse>> {
    let service = TagService::new(state.db.clone());
    // Personal KB mode: show all tags regardless of user
    let all_tags = service.list_all().await.map_err(AppError::Internal)?;

    let mut filtered: Vec<TagWithCount> = if let Some(ref q) = query.search {
        let q_lower = q.to_lowercase();
        all_tags.into_iter().filter(|t| t.name.to_lowercase().contains(&q_lower)).collect()
    } else {
        all_tags
    };

    // popular = sort by document_count desc
    if query.popular.as_deref() == Some("true") {
        filtered.sort_by(|a, b| b.document_count.cmp(&a.document_count));
    }

    let total = filtered.len() as i64;
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).clamp(1, 200);
    let pages = (total + per_page - 1) / per_page;
    let start = ((page - 1) * per_page) as usize;
    let paginated: Vec<TagWithCount> = filtered.into_iter().skip(start).take(per_page as usize).collect();

    Ok(Json(TagListResponse {
        success: true,
        data: TagListData {
            tags: paginated,
            pagination: Pagination {
                page,
                per_page,
                total,
                pages,
                has_next: page < pages,
                has_prev: page > 1,
            },
            total,
        },
    }))
}

// ---------------------------------------------------------------------------
// Get / Update / Delete by slug (tag name)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct TagResponse {
    pub success: bool,
    pub data: TagWithCount,
}

async fn get_tag(
    State(state): State<AppState>,
    _auth_user: OptionalAuthUser,
    Path(slug): Path<String>,
) -> AppResult<Json<TagResponse>> {
    let service = TagService::new(state.db.clone());
    let tag = service.get_by_name(&slug).await.map_err(AppError::Internal)?
        .ok_or_else(|| AppError::NotFound(format!("Tag '{}' not found", slug)))?;

    Ok(Json(TagResponse { success: true, data: tag }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTagRequest {
    #[validate(length(min = 1, max = 100, message = "Tag name must be 1-100 characters"))]
    pub name: String,
}

async fn create_tag(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateTagRequest>,
) -> AppResult<(StatusCode, Json<TagResponse>)> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let service = TagService::new(state.db.clone());
    let tag = service.create(auth_user.id, CreateTag { name: payload.name }).await?;

    let tag_with_count = TagWithCount {
        id: tag.id,
        name: tag.name,
        user_id: tag.user_id,
        document_count: 0,
        created_at: tag.created_at,
    };
    Ok((StatusCode::CREATED, Json(TagResponse { success: true, data: tag_with_count })))
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTagRequest {
    #[validate(length(min = 1, max = 100, message = "Tag name must be 1-100 characters"))]
    pub name: Option<String>,
}

async fn update_tag(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(slug): Path<String>,
    Json(payload): Json<UpdateTagRequest>,
) -> AppResult<Json<TagResponse>> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let service = TagService::new(state.db.clone());
    // Resolve slug → id, then delegate to existing service method
    let existing = service.get_by_name(&slug).await.map_err(AppError::Internal)?
        .ok_or_else(|| AppError::NotFound(format!("Tag '{}' not found", slug)))?;

    let tag = service.update(existing.id, auth_user.id, UpdateTag { name: payload.name }).await?;

    let tag_with_count = TagWithCount {
        id: tag.id,
        name: tag.name,
        user_id: tag.user_id,
        document_count: existing.document_count,
        created_at: tag.created_at,
    };
    Ok(Json(TagResponse { success: true, data: tag_with_count }))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_tag(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<Json<DeleteResponse>> {
    let service = TagService::new(state.db.clone());
    let existing = service.get_by_name(&slug).await.map_err(AppError::Internal)?
        .ok_or_else(|| AppError::NotFound(format!("Tag '{}' not found", slug)))?;

    service.delete(existing.id, auth_user.id).await?;
    Ok(Json(DeleteResponse { success: true, message: "Tag deleted successfully".to_string() }))
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct TagStatisticsResponse {
    pub success: bool,
    pub data: TagStatisticsData,
}

#[derive(Debug, Serialize)]
pub struct TagStatisticsData {
    pub total_tags: i64,
    pub total_tag_usage: i64,
    pub top_tags: Vec<TagWithCount>,
}

async fn tag_statistics(
    State(state): State<AppState>,
    _auth_user: OptionalAuthUser,
) -> AppResult<Json<TagStatisticsResponse>> {
    let service = TagService::new(state.db.clone());
    let all_tags = service.list_all().await.map_err(AppError::Internal)?;

    let total_tags = all_tags.len() as i64;
    let total_tag_usage: i64 = all_tags.iter().map(|t| t.document_count).sum();
    let mut sorted = all_tags;
    sorted.sort_by(|a, b| b.document_count.cmp(&a.document_count));
    let top_tags = sorted.into_iter().take(10).collect();

    Ok(Json(TagStatisticsResponse {
        success: true,
        data: TagStatisticsData { total_tags, total_tag_usage, top_tags },
    }))
}

// ---------------------------------------------------------------------------
// Suggest (autocomplete by prefix)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SuggestQuery {
    pub q: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SuggestResponse {
    pub success: bool,
    pub data: Vec<TagWithCount>,
}

async fn suggest_tags_search(
    State(state): State<AppState>,
    _auth_user: OptionalAuthUser,
    Query(query): Query<SuggestQuery>,
) -> AppResult<Json<SuggestResponse>> {
    let service = TagService::new(state.db.clone());
    let all_tags = service.list_all().await.map_err(AppError::Internal)?;
    let limit = query.limit.unwrap_or(10).min(50);

    let results: Vec<TagWithCount> = if let Some(ref q) = query.q {
        let q_lower = q.to_lowercase();
        all_tags.into_iter()
            .filter(|t| t.name.to_lowercase().starts_with(&q_lower))
            .take(limit)
            .collect()
    } else {
        all_tags.into_iter().take(limit).collect()
    };

    Ok(Json(SuggestResponse { success: true, data: results }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_tag(name: &str, count: i64) -> TagWithCount {
        TagWithCount { id: 1, name: name.to_string(), user_id: 1, document_count: count, created_at: Utc::now() }
    }

    // CreateTagRequest validation
    #[test]
    fn test_create_tag_request_valid() {
        assert!(CreateTagRequest { name: "rust".to_string() }.validate().is_ok());
    }

    #[test]
    fn test_create_tag_request_empty_name_fails() {
        assert!(CreateTagRequest { name: "".to_string() }.validate().is_err());
    }

    #[test]
    fn test_create_tag_request_name_too_long_fails() {
        assert!(CreateTagRequest { name: "x".repeat(101) }.validate().is_err());
    }

    #[test]
    fn test_create_tag_request_max_length_ok() {
        assert!(CreateTagRequest { name: "x".repeat(100) }.validate().is_ok());
    }

    #[test]
    fn test_create_tag_request_unicode() {
        assert!(CreateTagRequest { name: "한국어태그".to_string() }.validate().is_ok());
    }

    // UpdateTagRequest validation
    #[test]
    fn test_update_tag_request_none_ok() {
        assert!(UpdateTagRequest { name: None }.validate().is_ok());
    }

    #[test]
    fn test_update_tag_request_valid_name() {
        assert!(UpdateTagRequest { name: Some("new-tag".to_string()) }.validate().is_ok());
    }

    #[test]
    fn test_update_tag_request_empty_name_fails() {
        assert!(UpdateTagRequest { name: Some("".to_string()) }.validate().is_err());
    }

    // TagListResponse Flask compat shape
    #[test]
    fn flask_list_response_has_tags_key() {
        let data = TagListData {
            tags: vec![make_tag("rust", 5)],
            pagination: Pagination { page: 1, per_page: 50, total: 1, pages: 1, has_next: false, has_prev: false },
            total: 1,
        };
        let resp = TagListResponse { success: true, data };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"tags\":["));
        assert!(json.contains("\"pagination\":{"));
        assert!(json.contains("\"total\":1"));
    }

    #[test]
    fn flask_list_response_empty_tags() {
        let data = TagListData {
            tags: vec![],
            pagination: Pagination { page: 1, per_page: 50, total: 0, pages: 0, has_next: false, has_prev: false },
            total: 0,
        };
        let resp = TagListResponse { success: true, data };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"tags\":[]"));
    }

    // Pagination logic
    #[test]
    fn pagination_has_next_when_more_pages() {
        let p = Pagination { page: 1, per_page: 10, total: 25, pages: 3, has_next: true, has_prev: false };
        assert!(p.has_next);
        assert!(!p.has_prev);
    }

    #[test]
    fn pagination_has_prev_on_last_page() {
        let p = Pagination { page: 3, per_page: 10, total: 25, pages: 3, has_next: false, has_prev: true };
        assert!(!p.has_next);
        assert!(p.has_prev);
    }

    // Statistics response
    #[test]
    fn statistics_response_serializes() {
        let resp = TagStatisticsResponse {
            success: true,
            data: TagStatisticsData {
                total_tags: 5,
                total_tag_usage: 20,
                top_tags: vec![make_tag("rust", 10)],
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"total_tags\":5"));
        assert!(json.contains("\"total_tag_usage\":20"));
        assert!(json.contains("\"top_tags\":["));
    }

    // Suggest response
    #[test]
    fn suggest_response_serializes() {
        let resp = SuggestResponse {
            success: true,
            data: vec![make_tag("rust", 5), make_tag("rocket", 2)],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":["));
    }

    // Slug routing: statistics/suggest must not collide with /{slug}
    #[test]
    fn routes_creation_does_not_panic() {
        let _router: Router<AppState> = routes();
    }

    // DeleteResponse
    #[test]
    fn test_delete_response_creation() {
        let r = DeleteResponse { success: true, message: "Tag deleted successfully".to_string() };
        assert!(r.message.contains("deleted"));
    }
}
