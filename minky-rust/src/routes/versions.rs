use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppResult,
    models::{CreateVersion, VersionWithAuthor},
    services::{VersionDiff, VersionService},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/document/:document_id", get(list_versions).post(create_version))
        .route("/:id", get(get_version))
        .route("/document/:document_id/latest", get(get_latest_version))
        .route("/document/:document_id/restore/:version_number", post(restore_version))
        .route("/document/:document_id/compare", get(compare_versions))
}

#[derive(Debug, Serialize)]
pub struct VersionListResponse {
    pub success: bool,
    pub data: Vec<VersionWithAuthor>,
    pub count: i64,
}

async fn list_versions(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<VersionListResponse>> {
    let service = VersionService::new(state.db.clone());
    let versions = service.list_for_document(document_id).await?;
    let count = service.count_for_document(document_id).await?;

    Ok(Json(VersionListResponse {
        success: true,
        data: versions,
        count,
    }))
}

#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub success: bool,
    pub data: VersionData,
}

#[derive(Debug, Serialize)]
pub struct VersionData {
    pub id: i32,
    pub document_id: Uuid,
    pub content: String,
    pub version_number: i32,
    pub created_by: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn get_version(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> AppResult<Json<VersionResponse>> {
    let service = VersionService::new(state.db.clone());
    let version = service.get(id).await?;

    Ok(Json(VersionResponse {
        success: true,
        data: VersionData {
            id: version.id,
            document_id: version.document_id,
            content: version.content,
            version_number: version.version_number,
            created_by: version.created_by,
            created_at: version.created_at,
        },
    }))
}

async fn get_latest_version(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<VersionResponse>> {
    let service = VersionService::new(state.db.clone());
    let version = service.get_latest(document_id).await?;

    Ok(Json(VersionResponse {
        success: true,
        data: VersionData {
            id: version.id,
            document_id: version.document_id,
            content: version.content,
            version_number: version.version_number,
            created_by: version.created_by,
            created_at: version.created_at,
        },
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateVersionRequest {
    pub content: String,
}

async fn create_version(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
    Json(payload): Json<CreateVersionRequest>,
) -> AppResult<Json<VersionResponse>> {
    let user_id = 1; // TODO: Extract from JWT

    let service = VersionService::new(state.db.clone());
    let version = service
        .create(
            user_id,
            CreateVersion {
                document_id,
                content: payload.content,
            },
        )
        .await?;

    Ok(Json(VersionResponse {
        success: true,
        data: VersionData {
            id: version.id,
            document_id: version.document_id,
            content: version.content,
            version_number: version.version_number,
            created_by: version.created_by,
            created_at: version.created_at,
        },
    }))
}

async fn restore_version(
    State(state): State<AppState>,
    Path((document_id, version_number)): Path<(Uuid, i32)>,
) -> AppResult<Json<VersionResponse>> {
    let user_id = 1;

    let service = VersionService::new(state.db.clone());
    let version = service.restore(document_id, version_number, user_id).await?;

    Ok(Json(VersionResponse {
        success: true,
        data: VersionData {
            id: version.id,
            document_id: version.document_id,
            content: version.content,
            version_number: version.version_number,
            created_by: version.created_by,
            created_at: version.created_at,
        },
    }))
}

#[derive(Debug, Deserialize)]
pub struct CompareQuery {
    pub from: i32,
    pub to: i32,
}

#[derive(Debug, Serialize)]
pub struct CompareResponse {
    pub success: bool,
    pub data: VersionDiff,
}

async fn compare_versions(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
    axum::extract::Query(query): axum::extract::Query<CompareQuery>,
) -> AppResult<Json<CompareResponse>> {
    let service = VersionService::new(state.db.clone());

    let from_version = service.get_by_number(document_id, query.from).await?;
    let to_version = service.get_by_number(document_id, query.to).await?;

    let diff = VersionService::compare_versions(&from_version.content, &to_version.content);

    Ok(Json(CompareResponse {
        success: true,
        data: diff,
    }))
}
