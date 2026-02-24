use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{CreateVersion, VersionWithAuthor},
    services::{VersionDiff, VersionService},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/document/{document_id}", get(list_versions).post(create_version))
        .route("/{id}", get(get_version))
        .route("/document/{document_id}/latest", get(get_latest_version))
        .route("/document/{document_id}/restore/{version_number}", post(restore_version))
        .route("/document/{document_id}/compare", get(compare_versions))
}

#[derive(Debug, Serialize)]
pub struct VersionListResponse {
    pub success: bool,
    pub data: Vec<VersionWithAuthor>,
    pub count: i64,
}

async fn list_versions(
    State(state): State<AppState>,
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
    Json(payload): Json<CreateVersionRequest>,
) -> AppResult<Json<VersionResponse>> {
    let service = VersionService::new(state.db.clone());
    let version = service
        .create(
            auth_user.id,
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
    auth_user: AuthUser,
    Path((document_id, version_number)): Path<(Uuid, i32)>,
) -> AppResult<Json<VersionResponse>> {
    let service = VersionService::new(state.db.clone());
    let version = service.restore(document_id, version_number, auth_user.id).await?;

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
    _auth_user: AuthUser,
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // CreateVersionRequest tests
    #[test]
    fn test_create_version_request_with_content() {
        let req = CreateVersionRequest {
            content: "# Document\n\nSome content here.".to_string(),
        };
        assert!(!req.content.is_empty());
    }

    #[test]
    fn test_create_version_request_empty_content() {
        let req = CreateVersionRequest {
            content: "".to_string(),
        };
        assert!(req.content.is_empty());
    }

    #[test]
    fn test_create_version_request_long_content() {
        let req = CreateVersionRequest {
            content: "x".repeat(100000),
        };
        assert_eq!(req.content.len(), 100000);
    }

    // CompareQuery tests
    #[test]
    fn test_compare_query_valid() {
        let query = CompareQuery { from: 1, to: 3 };
        assert_eq!(query.from, 1);
        assert_eq!(query.to, 3);
    }

    #[test]
    fn test_compare_query_same_versions() {
        let query = CompareQuery { from: 2, to: 2 };
        assert_eq!(query.from, query.to);
    }

    #[test]
    fn test_compare_query_reverse_order() {
        let query = CompareQuery { from: 5, to: 1 };
        assert!(query.from > query.to);
    }

    // VersionListResponse tests
    #[test]
    fn test_version_list_response_creation() {
        let versions = vec![VersionWithAuthor {
            id: 1,
            document_id: Uuid::new_v4(),
            content: "Initial content".to_string(),
            version_number: 1,
            created_by: 10,
            author_name: "Alice".to_string(),
            created_at: Utc::now(),
        }];
        let response = VersionListResponse {
            success: true,
            data: versions,
            count: 1,
        };
        assert!(response.success);
        assert_eq!(response.count, 1);
        assert_eq!(response.data.len(), 1);
    }

    #[test]
    fn test_version_list_response_empty() {
        let response = VersionListResponse {
            success: true,
            data: vec![],
            count: 0,
        };
        assert!(response.data.is_empty());
        assert_eq!(response.count, 0);
    }

    #[test]
    fn test_version_list_response_multiple() {
        let doc_id = Uuid::new_v4();
        let versions = vec![
            VersionWithAuthor {
                id: 1,
                document_id: doc_id,
                content: "v1".to_string(),
                version_number: 1,
                created_by: 1,
                author_name: "Alice".to_string(),
                created_at: Utc::now(),
            },
            VersionWithAuthor {
                id: 2,
                document_id: doc_id,
                content: "v2".to_string(),
                version_number: 2,
                created_by: 2,
                author_name: "Bob".to_string(),
                created_at: Utc::now(),
            },
        ];
        let response = VersionListResponse {
            success: true,
            data: versions,
            count: 2,
        };
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].version_number, 1);
        assert_eq!(response.data[1].version_number, 2);
    }

    // VersionResponse tests
    #[test]
    fn test_version_response_creation() {
        let doc_id = Uuid::new_v4();
        let data = VersionData {
            id: 5,
            document_id: doc_id,
            content: "Document content".to_string(),
            version_number: 3,
            created_by: 42,
            created_at: Utc::now(),
        };
        let response = VersionResponse {
            success: true,
            data,
        };
        assert!(response.success);
        assert_eq!(response.data.id, 5);
        assert_eq!(response.data.version_number, 3);
    }

    // VersionData tests
    #[test]
    fn test_version_data_creation() {
        let doc_id = Uuid::new_v4();
        let data = VersionData {
            id: 1,
            document_id: doc_id,
            content: "Test content".to_string(),
            version_number: 1,
            created_by: 10,
            created_at: Utc::now(),
        };
        assert_eq!(data.id, 1);
        assert_eq!(data.document_id, doc_id);
        assert_eq!(data.version_number, 1);
    }

    #[test]
    fn test_version_data_high_version_number() {
        let data = VersionData {
            id: 100,
            document_id: Uuid::new_v4(),
            content: "Many versions".to_string(),
            version_number: 999,
            created_by: 1,
            created_at: Utc::now(),
        };
        assert_eq!(data.version_number, 999);
    }

    // VersionWithAuthor tests
    #[test]
    fn test_version_with_author_creation() {
        let version = VersionWithAuthor {
            id: 1,
            document_id: Uuid::new_v4(),
            content: "Content".to_string(),
            version_number: 1,
            created_by: 5,
            author_name: "Test User".to_string(),
            created_at: Utc::now(),
        };
        assert_eq!(version.author_name, "Test User");
        assert_eq!(version.created_by, 5);
    }

    // CompareResponse tests
    #[test]
    fn test_compare_response_creation() {
        let diff = VersionDiff {
            additions: 5,
            deletions: 2,
            total_changes: 7,
        };
        let response = CompareResponse {
            success: true,
            data: diff,
        };
        assert!(response.success);
        assert_eq!(response.data.additions, 5);
        assert_eq!(response.data.deletions, 2);
        assert_eq!(response.data.total_changes, 7);
    }

    #[test]
    fn test_compare_response_no_changes() {
        let diff = VersionDiff {
            additions: 0,
            deletions: 0,
            total_changes: 0,
        };
        let response = CompareResponse {
            success: true,
            data: diff,
        };
        assert_eq!(response.data.total_changes, 0);
    }
}
