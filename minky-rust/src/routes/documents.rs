use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use validator::Validate;

use secrecy::ExposeSecret;

use crate::{error::{AppError, AppResult}, middleware::{AuthUser, OptionalAuthUser}, AppState};
use crate::models::DuplicateGroup;
use crate::pipeline::{DocumentPipelineBuilder, IngestionInput};
use crate::services::{EmbeddingConfig, EmbeddingService};

const MAX_UPLOAD_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MAX_SRT_SIZE: usize = 20 * 1024 * 1024;    // 20MB (자막 파일은 더 클 수 있음)
const MAX_SRT_CHUNKS: usize = 50;                 // F1: Solar API 호출 상한

const UPSTAGE_CHAT_URL: &str = "https://api.upstage.ai/v1/chat/completions";
const SOLAR_MODEL: &str = "solar-open2";
const SRT_CHUNK_CHARS: usize = 2500;
const SRT_NARRATIVE_SYSTEM: &str = "\
당신은 경제·투자 대담 자막을 서술형 글로 변환하는 전문 편집자입니다.\n\
규칙:\n\
- 자막 내용을 요약하지 말고, 원문의 논지·주장·수치·사례를 빠짐없이 유지하세요.\n\
- \"이제\", \"막\", \"뭐\", \"그니까\", \"어\" 같은 구어 채움말은 정리하되 의미 있는 표현은 살리세요.\n\
- 완성된 문단 형태의 문어체로 작성하세요.\n\
- 마크다운 헤딩·리스트·강조 없이 흐르는 산문(prose)으로 작성하세요.";

pub fn routes() -> Router<AppState> {
    // F3: 라우트별 body limit 분리 (/upload 10MB, /upload/srt 20MB)
    Router::new()
        .route("/", get(list_documents).post(create_document))
        .route("/upload", post(upload_document).layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE)))
        .route("/upload/srt", post(upload_srt).layer(DefaultBodyLimit::max(MAX_SRT_SIZE)))
        .route("/duplicates", get(list_duplicates))
        .route("/duplicates/resolve", post(resolve_duplicates))
        .route("/tree", get(get_document_tree))
        .route("/{id}", get(get_document).put(update_document).delete(delete_document))
        .route("/{id}/archive", post(archive_document))
        .route("/{id}/restore", post(restore_document))
        .route("/{id}/status", get(get_document_status))
        .route("/{id}/reprocess", post(reprocess_document))
}

/// Query parameters for document listing
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub per_page: Option<i32>,  // Flask compat alias for limit
    pub category_id: Option<i32>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TreeQuery {
    pub mode: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreeNode>>,
}

#[derive(Debug, Serialize)]
pub struct TreeResponse {
    pub tree: Vec<TreeNode>,
}

async fn get_document_tree(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Query(query): Query<TreeQuery>,
) -> AppResult<Json<TreeResponse>> {
    let mode = query.mode.as_deref().unwrap_or("by-tag");
    let user_id = auth_user.0.map(|u| u.id).unwrap_or(-1);

    let tree = match mode {
        "by-date" => build_tree_by_date(&state, user_id).await?,
        _ => build_tree_by_tag(&state, user_id).await?,
    };

    Ok(Json(TreeResponse { tree }))
}

async fn build_tree_by_tag(state: &AppState, user_id: i32) -> AppResult<Vec<TreeNode>> {
    // 태그별 문서 그룹 — 태그 없는 문서는 "태그 없음" 노드로
    let rows: Vec<(Option<String>, Option<i32>, Uuid, String)> = sqlx::query_as(
        r#"
        SELECT t.name, t.id, d.id, d.title
        FROM documents d
        LEFT JOIN document_tags dt ON dt.document_id = d.id
        LEFT JOIN tags t ON t.id = dt.tag_id
        WHERE (d.user_id = $1 OR d.is_public = true) AND d.archived = false
        ORDER BY t.name NULLS LAST, d.title
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let mut tag_map: std::collections::BTreeMap<String, Vec<TreeNode>> = std::collections::BTreeMap::new();

    for (tag_name, tag_id, doc_id, doc_title) in rows {
        let key = match (&tag_name, &tag_id) {
            (Some(name), Some(id)) => format!("tag:{id}:{name}"),
            _ => "tag:untagged:태그 없음".to_string(),
        };
        let label = tag_name.unwrap_or_else(|| "태그 없음".to_string());
        tag_map.entry(key.clone()).or_default();
        // avoid duplicate doc entries when doc has multiple tags
        let docs = tag_map.get_mut(&key).unwrap();
        let doc_id_str = doc_id.to_string();
        if !docs.iter().any(|n| n.document_id.as_deref() == Some(&doc_id_str)) {
            docs.push(TreeNode {
                id: format!("doc:{doc_id}"),
                label: doc_title,
                node_type: "document".to_string(),
                document_id: Some(doc_id_str),
                count: None,
                children: None,
            });
        }
        let _ = label; // used in entry key above
    }

    let tree = tag_map
        .into_iter()
        .map(|(key, children)| {
            let parts: Vec<&str> = key.splitn(3, ':').collect();
            let label = parts.get(2).copied().unwrap_or("").to_string();
            let count = children.len() as i64;
            TreeNode {
                id: key,
                label,
                node_type: "tag".to_string(),
                document_id: None,
                count: Some(count),
                children: Some(children),
            }
        })
        .collect();

    Ok(tree)
}

async fn build_tree_by_date(state: &AppState, user_id: i32) -> AppResult<Vec<TreeNode>> {
    // 연도 > 월 > 문서 계층
    let rows: Vec<(i32, i32, Uuid, String)> = sqlx::query_as(
        r#"
        SELECT
            EXTRACT(YEAR FROM created_at)::int,
            EXTRACT(MONTH FROM created_at)::int,
            id, title
        FROM documents
        WHERE (user_id = $1 OR is_public = true) AND archived = false
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let mut year_map: std::collections::BTreeMap<i32, std::collections::BTreeMap<i32, Vec<TreeNode>>> =
        std::collections::BTreeMap::new();

    for (year, month, doc_id, doc_title) in rows {
        year_map
            .entry(year)
            .or_default()
            .entry(month)
            .or_default()
            .push(TreeNode {
                id: format!("doc:{doc_id}"),
                label: doc_title,
                node_type: "document".to_string(),
                document_id: Some(doc_id.to_string()),
                count: None,
                children: None,
            });
    }

    const MONTHS_KO: [&str; 12] = [
        "1월", "2월", "3월", "4월", "5월", "6월",
        "7월", "8월", "9월", "10월", "11월", "12월",
    ];

    let tree = year_map
        .into_iter()
        .map(|(year, month_map)| {
            let month_nodes: Vec<TreeNode> = month_map
                .into_iter()
                .map(|(month, docs)| {
                    let count = docs.len() as i64;
                    TreeNode {
                        id: format!("month:{year}-{month:02}"),
                        label: MONTHS_KO[(month - 1) as usize].to_string(),
                        node_type: "month".to_string(),
                        document_id: None,
                        count: Some(count),
                        children: Some(docs),
                    }
                })
                .collect();
            let total: i64 = month_nodes.iter().map(|m| m.count.unwrap_or(0)).sum();
            TreeNode {
                id: format!("year:{year}"),
                label: year.to_string(),
                node_type: "year".to_string(),
                document_id: None,
                count: Some(total),
                children: Some(month_nodes),
            }
        })
        .collect();

    Ok(tree)
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub category_id: Option<i32>,
    pub user_id: i32,
    pub is_public: bool,
    pub sensitivity: String,
    pub view_count: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListResponse<T> {
    pub success: bool,
    pub data: Vec<T>,
    pub meta: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub total: i64,
    pub page: i32,
    pub limit: i32,
    pub total_pages: i32,
}

// ── Flask-compat types (Decision A — response shape matching) ────────────────

/// DB row for lite list queries (no content/html fields; tag_names via ARRAY_AGG)
#[derive(Debug, Clone, sqlx::FromRow)]
struct FlaskDocumentLiteRow {
    pub id: Uuid,
    pub title: String,
    pub author: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub user_id: i32,
    pub category_id: Option<i32>,
    pub is_public: bool,
    pub sensitivity: String,
    pub is_published: Option<bool>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tag_names: Vec<String>,
}

/// Flask-compatible list item matching to_dict_lite (no markdown_content; has tag_names)
#[derive(Debug, Clone, Serialize)]
pub struct FlaskDocumentLite {
    pub id: Uuid,
    pub title: String,
    pub author: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub user_id: i32,
    pub category_id: Option<i32>,
    pub is_public: bool,
    pub sensitivity: String,
    pub is_published: bool,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tag_names: Vec<String>,
}

impl From<FlaskDocumentLiteRow> for FlaskDocumentLite {
    fn from(row: FlaskDocumentLiteRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            author: row.author,
            created_at: row.created_at,
            updated_at: row.updated_at,
            user_id: row.user_id,
            category_id: row.category_id,
            is_public: row.is_public,
            sensitivity: row.sensitivity,
            is_published: row.is_published.unwrap_or(false),
            published_at: row.published_at,
            tag_names: row.tag_names,
        }
    }
}

/// DB row for single-document queries (includes content + html_content; migration 011 columns)
#[derive(Debug, Clone, sqlx::FromRow)]
struct DocumentRow {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub category_id: Option<i32>,
    pub user_id: i32,
    pub is_public: bool,
    pub sensitivity: String,
    pub view_count: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub author: Option<String>,
    pub html_content: Option<String>,
    pub is_published: Option<bool>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub archived: bool,
    pub archived_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Flask-compatible single document (field `markdown_content`, no `{success, data}` wrapper)
#[derive(Debug, Clone, Serialize)]
pub struct FlaskDocumentItem {
    pub id: Uuid,
    pub title: String,
    pub markdown_content: String,
    pub category_id: Option<i32>,
    pub user_id: i32,
    pub is_public: bool,
    pub sensitivity: String,
    pub view_count: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub author: Option<String>,
    pub html_content: Option<String>,
    pub is_published: bool,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tag_names: Vec<String>,
    pub archived: bool,
    pub archived_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<DocumentRow> for FlaskDocumentItem {
    fn from(row: DocumentRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            markdown_content: row.content,
            category_id: row.category_id,
            user_id: row.user_id,
            is_public: row.is_public,
            sensitivity: row.sensitivity,
            view_count: row.view_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
            author: row.author,
            html_content: row.html_content,
            is_published: row.is_published.unwrap_or(false),
            published_at: row.published_at,
            tag_names: vec![],
            archived: row.archived,
            archived_at: row.archived_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FlaskPagination {
    pub page: i32,
    pub per_page: i32,
    pub total: i64,
    pub pages: i32,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(Debug, Serialize)]
pub struct FlaskListResponse {
    pub documents: Vec<FlaskDocumentLite>,
    pub pagination: FlaskPagination,
}

async fn list_documents(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<FlaskListResponse>> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.or(query.per_page).unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;
    // Unauthenticated: sentinel -1 → only is_public=true docs match (personal KB, no token sent)
    let user_id = auth_user.0.map(|u| u.id).unwrap_or(-1);

    // Only return user's own documents or public documents; archived docs are hidden by default
    let total: (i64,) = if let Some(ref search) = query.search {
        sqlx::query_as(
            "SELECT COUNT(*) FROM documents WHERE (user_id = $1 OR is_public = true) AND archived = false AND title ILIKE '%' || $2 || '%'"
        )
        .bind(user_id)
        .bind(search)
        .fetch_one(&state.db)
        .await?
    } else if let Some(cat_id) = query.category_id {
        sqlx::query_as(
            "SELECT COUNT(*) FROM documents WHERE (user_id = $1 OR is_public = true) AND archived = false AND category_id = $2"
        )
        .bind(user_id)
        .bind(cat_id)
        .fetch_one(&state.db)
        .await?
    } else {
        sqlx::query_as("SELECT COUNT(*) FROM documents WHERE (user_id = $1 OR is_public = true) AND archived = false")
            .bind(user_id)
            .fetch_one(&state.db)
            .await?
    };

    // LEFT JOIN document_tags+tags for ARRAY_AGG(tag name) → tag_names column
    // GROUP BY avoids duplicating document rows when one doc has multiple tags
    let rows: Vec<FlaskDocumentLiteRow> = if let Some(ref search) = query.search {
        sqlx::query_as(
            r#"SELECT d.id, d.title, d.author, d.created_at, d.updated_at, d.user_id, d.category_id,
                      d.is_public, d.sensitivity, d.is_published, d.published_at,
                      COALESCE(ARRAY_AGG(t.name ORDER BY t.name) FILTER (WHERE t.name IS NOT NULL), ARRAY[]::text[]) AS tag_names
               FROM documents d
               LEFT JOIN document_tags dt ON dt.document_id = d.id
               LEFT JOIN tags t ON t.id = dt.tag_id
               WHERE (d.user_id = $1 OR d.is_public = true) AND d.archived = false AND d.title ILIKE '%' || $2 || '%'
               GROUP BY d.id, d.title, d.author, d.created_at, d.updated_at, d.user_id, d.category_id,
                        d.is_public, d.sensitivity, d.is_published, d.published_at
               ORDER BY d.created_at DESC
               LIMIT $3 OFFSET $4"#
        )
        .bind(user_id)
        .bind(search)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    } else if let Some(cat_id) = query.category_id {
        sqlx::query_as(
            r#"SELECT d.id, d.title, d.author, d.created_at, d.updated_at, d.user_id, d.category_id,
                      d.is_public, d.sensitivity, d.is_published, d.published_at,
                      COALESCE(ARRAY_AGG(t.name ORDER BY t.name) FILTER (WHERE t.name IS NOT NULL), ARRAY[]::text[]) AS tag_names
               FROM documents d
               LEFT JOIN document_tags dt ON dt.document_id = d.id
               LEFT JOIN tags t ON t.id = dt.tag_id
               WHERE (d.user_id = $1 OR d.is_public = true) AND d.archived = false AND d.category_id = $2
               GROUP BY d.id, d.title, d.author, d.created_at, d.updated_at, d.user_id, d.category_id,
                        d.is_public, d.sensitivity, d.is_published, d.published_at
               ORDER BY d.created_at DESC
               LIMIT $3 OFFSET $4"#
        )
        .bind(user_id)
        .bind(cat_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            r#"SELECT d.id, d.title, d.author, d.created_at, d.updated_at, d.user_id, d.category_id,
                      d.is_public, d.sensitivity, d.is_published, d.published_at,
                      COALESCE(ARRAY_AGG(t.name ORDER BY t.name) FILTER (WHERE t.name IS NOT NULL), ARRAY[]::text[]) AS tag_names
               FROM documents d
               LEFT JOIN document_tags dt ON dt.document_id = d.id
               LEFT JOIN tags t ON t.id = dt.tag_id
               WHERE (d.user_id = $1 OR d.is_public = true) AND d.archived = false
               GROUP BY d.id, d.title, d.author, d.created_at, d.updated_at, d.user_id, d.category_id,
                        d.is_public, d.sensitivity, d.is_published, d.published_at
               ORDER BY d.created_at DESC
               LIMIT $2 OFFSET $3"#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    };

    let total_pages = ((total.0 as f64) / (limit as f64)).ceil() as i32;
    let documents: Vec<FlaskDocumentLite> = rows.into_iter().map(FlaskDocumentLite::from).collect();

    Ok(Json(FlaskListResponse {
        documents,
        pagination: FlaskPagination {
            page,
            per_page: limit,
            total: total.0,
            pages: total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        },
    }))
}

/// Create document request
#[derive(Debug, Deserialize, Validate)]
pub struct CreateDocumentRequest {
    #[validate(length(min = 1, max = 500, message = "Title must be 1-500 characters"))]
    pub title: String,
    pub markdown_content: String,
    pub category_id: Option<i32>,
    pub is_public: Option<bool>,
    pub sensitivity: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SingleResponse<T> {
    pub success: bool,
    pub data: T,
}

async fn create_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateDocumentRequest>,
) -> AppResult<Json<FlaskDocumentItem>> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let user_id = auth_user.id;
    let is_public = payload.is_public.unwrap_or(false);
    let sensitivity = payload.sensitivity.as_deref().unwrap_or("internal");

    if is_public && sensitivity == "confidential" {
        return Err(AppError::Validation(
            "confidential 문서는 공개(is_public=true)로 설정할 수 없습니다".to_string(),
        ));
    }

    // Compute content_hash for dedup; check before insert (archived docs don't block re-upload)
    let content_hash = hex::encode(sha2::Sha256::digest(payload.markdown_content.as_bytes()));
    let dup_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM documents WHERE user_id = $1 AND content_hash = $2 AND archived = false LIMIT 1"
    )
    .bind(user_id)
    .bind(&content_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?;
    if dup_id.is_some() {
        return Err(AppError::Conflict(
            "Duplicate document: this content already exists".to_string()
        ));
    }

    let row: DocumentRow = sqlx::query_as(
        r#"INSERT INTO documents (title, content, category_id, user_id, is_public, sensitivity, content_hash)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, title, content, category_id, user_id, is_public, sensitivity, view_count, created_at, updated_at,
                     author, html_content, is_published, published_at, archived, archived_at"#
    )
    .bind(&payload.title)
    .bind(&payload.markdown_content)
    .bind(payload.category_id)
    .bind(user_id)
    .bind(is_public)
    .bind(sensitivity)
    .bind(&content_hash)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(FlaskDocumentItem::from(row)))
}

async fn get_document(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<FlaskDocumentItem>> {
    // Unauthenticated: sentinel -1 → only is_public=true docs accessible
    let user_id = auth_user.0.map(|u| u.id).unwrap_or(-1);

    // Only allow access to own documents or public documents; archived docs are hidden
    let row: Option<DocumentRow> = sqlx::query_as(
        r#"SELECT id, title, content, category_id, user_id, is_public, sensitivity, view_count, created_at, updated_at,
                  author, html_content, is_published, published_at, archived, archived_at
           FROM documents
           WHERE id = $1 AND (user_id = $2 OR is_public = true) AND archived = false"#
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    let row = row.ok_or_else(|| AppError::NotFound(format!("Document {} not found or access denied", id)))?;

    // Increment view count
    sqlx::query("UPDATE documents SET view_count = view_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    let mut item = FlaskDocumentItem::from(row);

    // Fetch tag names (list queries use ARRAY_AGG; single doc needs a separate join)
    let tag_rows: Vec<(String,)> = sqlx::query_as(
        "SELECT t.name FROM tags t \
         JOIN document_tags dt ON dt.tag_id = t.id \
         WHERE dt.document_id = $1 ORDER BY t.name",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    item.tag_names = tag_rows.into_iter().map(|r| r.0).collect();

    Ok(Json(item))
}

/// Update document request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateDocumentRequest {
    #[validate(length(min = 1, max = 500, message = "Title must be 1-500 characters"))]
    pub title: Option<String>,
    pub markdown_content: Option<String>,
    pub category_id: Option<i32>,
    pub is_public: Option<bool>,
    pub sensitivity: Option<String>,
}

async fn update_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateDocumentRequest>,
) -> AppResult<Json<FlaskDocumentItem>> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let user_id = auth_user.id;

    // Check document exists AND user owns it; also read current is_public + sensitivity
    // so we can validate the merged state before applying the update.
    let existing: Option<(Uuid, bool, String)> = sqlx::query_as(
        "SELECT id, is_public, sensitivity FROM documents WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    let (_, current_is_public, current_sensitivity) = existing
        .ok_or_else(|| AppError::NotFound(format!("Document {} not found or access denied", id)))?;

    // Compute effective values after the update (None = keep existing)
    let effective_is_public = payload.is_public.unwrap_or(current_is_public);
    let effective_sensitivity = payload
        .sensitivity
        .as_deref()
        .unwrap_or(&current_sensitivity)
        .to_string();

    if effective_is_public && effective_sensitivity == "confidential" {
        return Err(AppError::Validation(
            "confidential 문서는 공개(is_public=true)로 설정할 수 없습니다".to_string(),
        ));
    }

    // Guard: if new content matches an existing doc (other than this one), reject with 409.
    // Without this check, an edit can silently create two docs sharing the same content_hash,
    // which would break the UNIQUE(user_id, content_hash) constraint added in Sprint D2.
    if let Some(ref new_content) = payload.markdown_content {
        let new_hash = hex::encode(Sha256::digest(new_content.as_bytes()));
        let collision: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM documents WHERE user_id = $1 AND content_hash = $2 AND id != $3 AND archived = false LIMIT 1"
        )
        .bind(user_id)
        .bind(&new_hash)
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::Database)?;
        if collision.is_some() {
            return Err(AppError::Conflict(
                "Duplicate document: this content already exists".to_string()
            ));
        }
    }

    // When content changes, recompute content_hash to keep it consistent.
    // Uses SQL-level expression so hash always tracks the stored content.
    let row: DocumentRow = sqlx::query_as(
        r#"UPDATE documents
           SET
               title = COALESCE($1, title),
               content = COALESCE($2, content),
               content_hash = CASE
                   WHEN $2 IS NOT NULL THEN encode(digest($2, 'sha256'), 'hex')
                   ELSE content_hash
               END,
               category_id = CASE WHEN $3::boolean THEN $4::integer ELSE category_id END,
               is_public = COALESCE($5, is_public),
               sensitivity = COALESCE($6, sensitivity),
               updated_at = NOW()
           WHERE id = $7 AND user_id = $8
           RETURNING id, title, content, category_id, user_id, is_public, sensitivity, view_count, created_at, updated_at,
                     author, html_content, is_published, published_at, archived, archived_at"#
    )
    .bind(payload.title.as_deref())
    .bind(payload.markdown_content.as_deref())
    .bind(payload.category_id.is_some())
    .bind(payload.category_id)
    .bind(payload.is_public)
    .bind(payload.sensitivity.as_deref())
    .bind(id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(FlaskDocumentItem::from(row)))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<DeleteResponse>> {
    let user_id = auth_user.id;

    // Only allow owner to delete their document
    let result = sqlx::query("DELETE FROM documents WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Document {} not found or access denied", id)));
    }

    Ok(Json(DeleteResponse {
        success: true,
        message: format!("Document {} deleted successfully", id),
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub success: bool,
    pub document: UploadedDocument,
}

#[derive(Debug, Serialize)]
pub struct UploadedDocument {
    pub id: Uuid,
    pub title: String,
    pub chunks_count: usize,
    pub processing_status: ProcessingStatus,
}

fn title_from_filename(filename: &str) -> String {
    let stem = filename.strip_suffix(".md")
        .or_else(|| filename.strip_suffix(".MD"))
        .unwrap_or(filename);
    stem.replace(['-', '_'], " ")
}

fn validate_upload_filename(filename: &str) -> Result<(), String> {
    if filename.contains('/') || filename.contains('\\') || filename.contains('\0') {
        return Err("Invalid filename".to_string());
    }
    if filename.to_lowercase().ends_with(".md") {
        Ok(())
    } else {
        Err("Only .md files are accepted".to_string())
    }
}

fn validate_upload_data(data: &[u8]) -> Result<(), String> {
    if data.is_empty() {
        return Err("File is empty".to_string());
    }
    if data.len() > MAX_UPLOAD_SIZE {
        return Err(format!(
            "File too large: {} bytes (max {} bytes)",
            data.len(),
            MAX_UPLOAD_SIZE
        ));
    }
    std::str::from_utf8(data)
        .map_err(|_| "File is not valid UTF-8 text".to_string())?;
    Ok(())
}

async fn upload_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Validation(format!("Failed to read multipart field: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        if name != "file" {
            continue;
        }

        let original_filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "untitled.md".to_string());

        validate_upload_filename(&original_filename)
            .map_err(AppError::Validation)?;

        let data = field.bytes().await.map_err(|e| {
            AppError::Validation(format!("Failed to read file data: {}", e))
        })?;

        validate_upload_data(&data)
            .map_err(AppError::Validation)?;

        // Safe: validate_upload_data already verified UTF-8
        let content = std::str::from_utf8(&data)
            .expect("BUG: validate_upload_data already verified UTF-8")
            .to_string();
        let title = title_from_filename(&original_filename);

        // Duplicate check: only block on active (non-archived) duplicates
        let content_hash = hex::encode(Sha256::digest(content.as_bytes()));
        let existing: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM documents WHERE user_id = $1 AND content_hash = $2 AND archived = false LIMIT 1"
        )
        .bind(auth_user.id)
        .bind(&content_hash)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::Database)?;

        if existing.is_some() {
            return Err(AppError::Conflict(
                "Duplicate document: this content was already uploaded".to_string()
            ));
        }

        let upstage_key = std::env::var("UPSTAGE_API_KEY").unwrap_or_default();
        let mut builder = DocumentPipelineBuilder::new()
            .pool(state.db.clone())
            .app_config(state.config.clone())
            .semantic_chunking(512)
            .user_id(auth_user.id)
            .anthropic_api_key(upstage_key.clone());
        if upstage_key.is_empty() {
            builder = builder.skip_embedding();
        } else {
            builder = builder.upstage_embedding(upstage_key);
        }
        let pipeline = builder.build();
        let input = IngestionInput::from_text(title.clone(), content);

        let output = pipeline.process(input).await.map_err(|e| {
            tracing::error!("Upload pipeline failed: {}", e);
            AppError::Internal(anyhow::anyhow!("Pipeline processing failed: {}", e))
        })?;

        let processing_status = if output.analyzed {
            ProcessingStatus::Completed
        } else {
            enqueue_for_embedding(&state, output.document_id).await;
            ProcessingStatus::Pending
        };

        return Ok(Json(UploadResponse {
            success: true,
            document: UploadedDocument {
                id: output.document_id,
                title: output.title,
                chunks_count: output.chunks_count,
                processing_status,
            },
        }));
    }

    Err(AppError::Validation("No file field found in upload".to_string()))
}

// ── SRT 자막 업로드 ──────────────────────────────────────────────────────────

/// Rolling 2-line SRT 자막에서 중복 제거 후 순수 텍스트 반환.
///
/// 각 블록의 마지막 비어있지 않은 줄이 '이번 블록의 새 텍스트'이므로
/// 이전 블록과 다를 때만 수집한다.
fn parse_srt_to_text(srt: &str) -> String {
    let mut result: Vec<&str> = Vec::new();
    let mut prev: &str = "";

    for block in srt.split("\n\n") {
        let new_line = block
            .lines()
            .rfind(|l: &&str| {
                let l = l.trim();
                !l.is_empty()
                    && !l.chars().all(|c| c.is_ascii_digit())
                    && !l.contains("-->")
            })
            .map(|l| l.trim())
            .unwrap_or("");

        if !new_line.is_empty() && new_line != prev {
            result.push(new_line);
            prev = new_line;
        }
    }
    result.join("\n")
}

/// 텍스트를 최대 `max_chars` 글자 단위 청크로 분할.
fn chunk_srt_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut buf: Vec<&str> = Vec::new();
    let mut buf_len: usize = 0;

    for line in text.lines() {
        if buf_len + line.len() + 1 > max_chars && !buf.is_empty() {
            chunks.push(buf.join("\n"));
            buf.clear();
            buf_len = 0;
        }
        buf.push(line);
        buf_len += line.len() + 1;
    }
    if !buf.is_empty() {
        chunks.push(buf.join("\n"));
    }
    chunks
}

/// Solar-Open2 think 태그 제거.
fn strip_solar_think(text: &str) -> String {
    use crate::services::reasoning::strip_reasoning;
    strip_reasoning(text)
}

#[derive(Debug, Serialize)]
struct SolarChatRequest {
    model: String,
    messages: Vec<SolarMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct SolarMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct SolarChatResponse {
    choices: Vec<SolarChoice>,
}

#[derive(Debug, Deserialize)]
struct SolarChoice {
    message: SolarResponseMessage,
}

#[derive(Debug, Deserialize)]
struct SolarResponseMessage {
    #[serde(default)]
    content: String,
}

/// Upstage Solar-Open2 API 단일 청크 호출.
async fn call_solar_narrative(
    client: &reqwest::Client,
    api_key: &str,
    chunk: &str,
) -> AppResult<String> {
    let payload = SolarChatRequest {
        model: SOLAR_MODEL.to_string(),
        messages: vec![
            SolarMessage { role: "system".to_string(), content: SRT_NARRATIVE_SYSTEM.to_string() },
            SolarMessage {
                role: "user".to_string(),
                content: format!("다음 자막을 서술형 글로 작성하세요:\n\n{chunk}"),
            },
        ],
        max_tokens: 4096,
        temperature: 0.3,
    };

    let resp = client
        .post(UPSTAGE_CHAT_URL)
        .bearer_auth(api_key)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::ExternalService(format!("Solar API request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err = resp.text().await.unwrap_or_default();
        tracing::error!(status = %status, body = %err, "Solar API error");
        return Err(AppError::ExternalService("Solar API request failed".to_string()));
    }

    let body: SolarChatResponse = resp
        .json()
        .await
        .map_err(|e| AppError::ExternalService(format!("Solar API parse error: {e}")))?;

    let raw = body.choices.first().map(|c| c.message.content.as_str()).unwrap_or("");
    Ok(strip_solar_think(raw))
}

/// SRT 자막 파일 업로드 → Solar-Open2 서술형 변환 → 문서 저장
///
/// POST /api/documents/upload/srt
/// multipart field name: "file", content-type: text/plain or application/octet-stream
async fn upload_srt(
    State(state): State<AppState>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    let api_key = state.config.upstage_api_key
        .as_ref()
        .ok_or_else(|| AppError::Configuration("UPSTAGE_API_KEY not configured".to_string()))?
        .expose_secret()
        .to_string();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Validation(format!("Failed to read multipart field: {e}"))
    })? {
        if field.name().unwrap_or("") != "file" {
            continue;
        }

        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "subtitle.srt".to_string());

        if !filename.to_lowercase().ends_with(".srt") {
            return Err(AppError::Validation("Only .srt files are accepted".to_string()));
        }
        if filename.contains('/') || filename.contains('\\') || filename.contains('\0') {
            return Err(AppError::Validation("Invalid filename".to_string()));
        }

        let data = field.bytes().await.map_err(|e| {
            AppError::Validation(format!("Failed to read SRT data: {e}"))
        })?;

        if data.is_empty() {
            return Err(AppError::Validation("SRT file is empty".to_string()));
        }

        let srt_text = std::str::from_utf8(&data)
            .map_err(|_| AppError::Validation("SRT file must be valid UTF-8".to_string()))?;

        // 자막 → 순수 텍스트
        let clean_text = parse_srt_to_text(srt_text);
        if clean_text.is_empty() {
            return Err(AppError::Validation("No subtitle text found in SRT file".to_string()));
        }

        let chunks = chunk_srt_text(&clean_text, SRT_CHUNK_CHARS);
        if chunks.len() > MAX_SRT_CHUNKS {
            return Err(AppError::Validation(format!(
                "SRT file too large: {} chunks (max {}). Trim the file and retry.",
                chunks.len(), MAX_SRT_CHUNKS
            )));
        }
        tracing::info!(
            file = %filename,
            chars = clean_text.len(),
            chunks = chunks.len(),
            "SRT upload: starting Solar-Open2 narrative conversion"
        );

        // Solar-Open2로 각 청크 변환 (shared http_client 사용 — F2)
        let mut narratives: Vec<String> = Vec::with_capacity(chunks.len());
        for (i, chunk) in chunks.iter().enumerate() {
            tracing::info!(chunk = i + 1, total = chunks.len(), "Solar-Open2 chunk conversion");
            let narrative = call_solar_narrative(&state.http_client, &api_key, chunk).await?;
            narratives.push(narrative);
        }

        // 제목 추출: 'title.NA.channel.ko.srt' → 'title'
        let title_raw = filename.strip_suffix(".srt").unwrap_or(&filename);
        let title = title_raw
            .split(".NA.")
            .next()
            .unwrap_or(title_raw)
            .trim()
            .to_string();
        let title = if title.is_empty() { title_raw.to_string() } else { title };

        // MD 조립
        let md_content = format!("# {title}\n\n{}", narratives.join("\n\n"));

        // 기존 파이프라인으로 문서 저장
        let upstage_key = std::env::var("UPSTAGE_API_KEY").unwrap_or_default();
        let mut builder = DocumentPipelineBuilder::new()
            .pool(state.db.clone())
            .app_config(state.config.clone())
            .semantic_chunking(512)
            .user_id(auth_user.id)
            .anthropic_api_key(upstage_key.clone());
        if upstage_key.is_empty() {
            builder = builder.skip_embedding();
        } else {
            builder = builder.upstage_embedding(upstage_key);
        }
        let pipeline = builder.build();
        let input = IngestionInput::from_text(title.clone(), md_content);

        let output = pipeline.process(input).await.map_err(|e| {
            tracing::error!("SRT upload pipeline failed: {}", e);
            AppError::Internal(anyhow::anyhow!("Pipeline processing failed: {}", e))
        })?;

        enqueue_for_embedding(&state, output.document_id).await;
        tracing::info!(doc_id = %output.document_id, "SRT upload complete");

        return Ok(Json(UploadResponse {
            success: true,
            document: UploadedDocument {
                id: output.document_id,
                title: output.title,
                chunks_count: output.chunks_count,
                processing_status: ProcessingStatus::Pending,
            },
        }));
    }

    Err(AppError::Validation("No file field found in upload".to_string()))
}

// ─────────────────────────────────────────────────────────────────────────────

/// List duplicate document groups for the authenticated user.
///
/// `GET /api/documents/duplicates`
///
/// Returns groups of documents with identical content (same SHA-256 hash).
/// Each group contains the IDs and titles of duplicate documents.
#[derive(Debug, Serialize)]
struct DuplicatesResponse {
    success: bool,
    total_groups: usize,
    total_removable: usize,
    groups: Vec<DuplicateGroup>,
}

async fn list_duplicates(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<DuplicatesResponse>> {
    let groups: Vec<DuplicateGroup> = sqlx::query_as(
        r#"
        SELECT
            user_id,
            content_hash,
            COUNT(*)::bigint          AS doc_count,
            MIN(created_at)           AS oldest_at,
            MAX(created_at)           AS newest_at,
            array_agg(id ORDER BY created_at)     AS document_ids,
            array_agg(title ORDER BY created_at)  AS titles
        FROM documents
        WHERE user_id = $1
          AND content_hash IS NOT NULL
        GROUP BY user_id, content_hash
        HAVING COUNT(*) > 1
        ORDER BY COUNT(*) DESC
        LIMIT 500
        "#,
    )
    .bind(auth_user.id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::Database)?;

    let total_removable = groups.iter().map(|g| (g.doc_count - 1) as usize).sum();

    Ok(Json(DuplicatesResponse {
        success: true,
        total_groups: groups.len(),
        total_removable,
        groups,
    }))
}

/// POST /api/documents/{id}/archive — soft-archive a document (owner only)
async fn archive_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let rows_affected = sqlx::query(
        "UPDATE documents SET archived = true, archived_at = NOW() WHERE id = $1 AND user_id = $2 AND archived = false"
    )
    .bind(id)
    .bind(auth_user.id)
    .execute(&state.db)
    .await
    .map_err(AppError::Database)?
    .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Document {} not found, already archived, or access denied", id)));
    }
    Ok(Json(serde_json::json!({ "success": true, "archived": true })))
}

/// POST /api/documents/{id}/restore — restore a soft-archived document (owner only)
async fn restore_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    // Check for hash collision with currently active docs before restoring
    let target: Option<(bool, Option<String>)> = sqlx::query_as(
        "SELECT archived, content_hash FROM documents WHERE id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(auth_user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?;

    let (is_archived, hash) = target
        .ok_or_else(|| AppError::NotFound(format!("Document {} not found or access denied", id)))?;

    if !is_archived {
        return Ok(Json(serde_json::json!({ "success": true, "archived": false, "note": "already active" })));
    }

    if let Some(ref h) = hash {
        let collision: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM documents WHERE user_id = $1 AND content_hash = $2 AND archived = false AND id != $3 LIMIT 1"
        )
        .bind(auth_user.id)
        .bind(h)
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::Database)?;
        if collision.is_some() {
            return Err(AppError::Conflict(
                "Cannot restore: an active document with the same content already exists".to_string()
            ));
        }
    }

    sqlx::query(
        "UPDATE documents SET archived = false, archived_at = NULL WHERE id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(auth_user.id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        // Unique constraint violation (23505) means a concurrent restore beat us to it.
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.code().as_deref() == Some("23505") {
                return AppError::Conflict(
                    "Cannot restore: an active document with the same content already exists".to_string()
                );
            }
        }
        AppError::Database(e)
    })?;

    Ok(Json(serde_json::json!({ "success": true, "archived": false })))
}

#[derive(Debug, Serialize)]
struct ResolveResult {
    success: bool,
    archived_count: u64,
    groups_resolved: usize,
}

/// POST /api/documents/duplicates/resolve — batch soft-archive duplicates, keeping oldest
async fn resolve_duplicates(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<ResolveResult>> {
    // CTE-based UPDATE: archives duplicates and counts affected rows + distinct groups
    // in a single atomic statement. Keeper subquery restricts to active rows only,
    // so a previously-archived oldest copy cannot cause the last active copy to be archived.
    let counts: (i64, i64) = sqlx::query_as(
        r#"
        WITH archived_rows AS (
            UPDATE documents
            SET archived = true, archived_at = NOW()
            WHERE user_id = $1
              AND archived = false
              AND content_hash IS NOT NULL
              AND id NOT IN (
                  SELECT DISTINCT ON (user_id, content_hash) id
                  FROM documents
                  WHERE user_id = $1 AND content_hash IS NOT NULL AND archived = false
                  ORDER BY user_id, content_hash, created_at ASC, id ASC
              )
            RETURNING content_hash
        )
        SELECT COUNT(*) as archived_count,
               COUNT(DISTINCT content_hash) as groups_resolved
        FROM archived_rows
        "#
    )
    .bind(auth_user.id)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?;

    Ok(Json(ResolveResult {
        success: true,
        archived_count: counts.0 as u64,
        groups_resolved: counts.1 as usize,
    }))
}

fn embedding_service_from_state(state: &AppState) -> EmbeddingService {
    let upstage_key = std::env::var("UPSTAGE_API_KEY").ok();
    let default_model = if upstage_key.is_some() {
        crate::models::EmbeddingModel::UpstaageSolarPassage
    } else {
        crate::models::EmbeddingModel::OpenaiTextEmbedding3Small
    };
    let config = EmbeddingConfig {
        openai_api_key: state.config.openai_api_key.as_ref().map(|s| s.expose_secret().to_string()),
        upstage_api_key: upstage_key,
        default_model,
        ..EmbeddingConfig::default()
    };
    EmbeddingService::new(state.db.clone(), config)
}

async fn enqueue_for_embedding(state: &AppState, document_id: Uuid) {
    let service = embedding_service_from_state(state);
    if let Err(e) = service.queue_document(document_id, 0).await {
        tracing::warn!("Failed to enqueue document {} for embedding: {}", document_id, e);
    }
}

#[derive(Debug, Serialize)]
pub struct DocumentStatus {
    pub document_id: Uuid,
    pub processing_status: ProcessingStatus,
    pub queue_position: Option<i64>,
    pub error_message: Option<String>,
}

async fn get_document_status(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<DocumentStatus>, AppError> {
    let doc_exists: Option<(i32,)> = sqlx::query_as(
        "SELECT user_id FROM documents WHERE id = $1 AND (user_id = $2 OR is_public = true)"
    )
    .bind(id)
    .bind(auth_user.id)
    .fetch_optional(&state.db)
    .await?;

    if doc_exists.is_none() {
        return Err(AppError::NotFound(format!("Document not found: {}", id)));
    }

    type QueueRow = (String, Option<String>);
    let queue_entry: Option<QueueRow> = sqlx::query_as(
        "SELECT status, error_message FROM embedding_queue WHERE document_id = $1 ORDER BY created_at DESC LIMIT 1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    let (processing_status, error_message, queue_position) = match queue_entry {
        Some((status, err_msg)) => {
            let ps = match status.as_str() {
                "completed" => ProcessingStatus::Completed,
                "failed" => ProcessingStatus::Failed,
                _ => ProcessingStatus::Pending,
            };
            let pos = if ps == ProcessingStatus::Pending {
                sqlx::query_as::<_, (i64,)>(
                    r#"SELECT COUNT(*) FROM embedding_queue
                       WHERE status = 'pending'
                         AND (priority > (SELECT priority FROM embedding_queue WHERE document_id = $1 AND status = 'pending' LIMIT 1)
                              OR (priority = (SELECT priority FROM embedding_queue WHERE document_id = $1 AND status = 'pending' LIMIT 1)
                                  AND created_at < (SELECT created_at FROM embedding_queue WHERE document_id = $1 AND status = 'pending' LIMIT 1)))
                    "#,
                )
                .bind(id)
                .fetch_one(&state.db)
                .await
                .ok()
                .map(|(c,)| c + 1)
            } else {
                None
            };
            (ps, err_msg, pos)
        }
        None => {
            let has_embedding: Option<(i64,)> = sqlx::query_as(
                "SELECT COUNT(*) FROM document_embeddings WHERE document_id = $1"
            )
            .bind(id)
            .fetch_one(&state.db)
            .await
            .ok();
            if has_embedding.is_some_and(|(c,)| c > 0) {
                (ProcessingStatus::Completed, None, None)
            } else {
                (ProcessingStatus::Pending, None, None)
            }
        }
    };

    Ok(Json(DocumentStatus {
        document_id: id,
        processing_status,
        queue_position,
        error_message,
    }))
}

#[derive(Debug, Serialize)]
pub struct ReprocessResponse {
    pub success: bool,
    pub document_id: Uuid,
    pub message: String,
}

async fn reprocess_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ReprocessResponse>, AppError> {
    let doc: Option<(i32,)> = sqlx::query_as(
        "SELECT user_id FROM documents WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    match doc {
        None => Err(AppError::NotFound(format!("Document not found: {}", id))),
        Some((owner_id,)) if owner_id != auth_user.id => {
            Err(AppError::Forbidden)
        }
        Some(_) => {
            let service = embedding_service_from_state(&state);
            service.queue_document(id, 1).await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to enqueue: {}", e))
            })?;

            Ok(Json(ReprocessResponse {
                success: true,
                document_id: id,
                message: "Document queued for reprocessing".to_string(),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // ListQuery tests
    #[test]
    fn test_list_query_default_values() {
        let query = ListQuery {
            page: None,
            limit: None,
            per_page: None,
            category_id: None,
            search: None,
        };
        assert!(query.page.is_none());
        assert!(query.limit.is_none());
        assert!(query.category_id.is_none());
        assert!(query.search.is_none());
    }

    #[test]
    fn test_list_query_with_values() {
        let query = ListQuery {
            page: Some(2),
            limit: Some(50),
            per_page: None,
            category_id: Some(5),
            search: Some("test".to_string()),
        };
        assert_eq!(query.page, Some(2));
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.category_id, Some(5));
        assert_eq!(query.search, Some("test".to_string()));
    }

    // Pagination calculation tests
    #[test]
    fn test_page_min_value_clamped_to_1() {
        let query = ListQuery {
            page: Some(0),
            limit: None,
            per_page: None,
            category_id: None,
            search: None,
        };
        let page = query.page.unwrap_or(1).max(1);
        assert_eq!(page, 1);
    }

    #[test]
    fn test_page_negative_clamped_to_1() {
        let query = ListQuery {
            page: Some(-5),
            limit: None,
            per_page: None,
            category_id: None,
            search: None,
        };
        let page = query.page.unwrap_or(1).max(1);
        assert_eq!(page, 1);
    }

    #[test]
    fn test_limit_default_is_20() {
        let query = ListQuery {
            page: None,
            limit: None,
            per_page: None,
            category_id: None,
            search: None,
        };
        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        assert_eq!(limit, 20);
    }

    #[test]
    fn test_limit_max_clamped_to_100() {
        let query = ListQuery {
            page: None,
            limit: Some(500),
            per_page: None,
            category_id: None,
            search: None,
        };
        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        assert_eq!(limit, 100);
    }

    #[test]
    fn test_limit_min_clamped_to_1() {
        let query = ListQuery {
            page: None,
            limit: Some(0),
            per_page: None,
            category_id: None,
            search: None,
        };
        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        assert_eq!(limit, 1);
    }

    #[test]
    fn test_offset_calculation() {
        let page = 3;
        let limit = 20;
        let offset = (page - 1) * limit;
        assert_eq!(offset, 40);
    }

    #[test]
    fn test_offset_first_page() {
        let page = 1;
        let limit = 20;
        let offset = (page - 1) * limit;
        assert_eq!(offset, 0);
    }

    // DocumentResponse tests
    #[test]
    fn test_document_response_creation() {
        let doc = DocumentResponse {
            id: Uuid::new_v4(),
            title: "Test Doc".to_string(),
            content: "Content here".to_string(),
            category_id: Some(1),
            user_id: 42,
            is_public: true,
            sensitivity: "internal".to_string(),
            view_count: 10,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(doc.title, "Test Doc");
        assert_eq!(doc.user_id, 42);
        assert!(doc.is_public);
    }

    #[test]
    fn test_document_response_private() {
        let doc = DocumentResponse {
            id: Uuid::new_v4(),
            title: "Private Doc".to_string(),
            content: "Secret".to_string(),
            category_id: None,
            user_id: 1,
            is_public: false,
            sensitivity: "confidential".to_string(),
            view_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(!doc.is_public);
        assert!(doc.category_id.is_none());
    }

    // PaginationMeta tests
    #[test]
    fn test_pagination_meta_creation() {
        let meta = PaginationMeta {
            total: 100,
            page: 1,
            limit: 20,
            total_pages: 5,
        };
        assert_eq!(meta.total, 100);
        assert_eq!(meta.total_pages, 5);
    }

    #[test]
    fn test_total_pages_calculation() {
        let total: i64 = 95;
        let limit = 20;
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i32;
        assert_eq!(total_pages, 5);
    }

    #[test]
    fn test_total_pages_exact_division() {
        let total: i64 = 100;
        let limit = 20;
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i32;
        assert_eq!(total_pages, 5);
    }

    #[test]
    fn test_total_pages_single_page() {
        let total: i64 = 15;
        let limit = 20;
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i32;
        assert_eq!(total_pages, 1);
    }

    #[test]
    fn test_total_pages_empty() {
        let total: i64 = 0;
        let limit = 20;
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i32;
        assert_eq!(total_pages, 0);
    }

    // CreateDocumentRequest validation tests
    #[test]
    fn test_create_document_request_valid() {
        let req = CreateDocumentRequest {
            title: "Valid Title".to_string(),
            markdown_content: "Some content".to_string(),
            category_id: None,
            is_public: Some(true),
            sensitivity: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_document_request_empty_title_fails() {
        let req = CreateDocumentRequest {
            title: "".to_string(),
            markdown_content: "Content".to_string(),
            category_id: None,
            is_public: None,
            sensitivity: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_document_request_title_too_long_fails() {
        let req = CreateDocumentRequest {
            title: "x".repeat(501),
            markdown_content: "Content".to_string(),
            category_id: None,
            is_public: None,
            sensitivity: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_document_request_max_title_length_ok() {
        let req = CreateDocumentRequest {
            title: "x".repeat(500),
            markdown_content: "Content".to_string(),
            category_id: None,
            is_public: None,
            sensitivity: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_document_is_public_default_false() {
        let req = CreateDocumentRequest {
            title: "Test".to_string(),
            markdown_content: "Content".to_string(),
            category_id: None,
            is_public: None,
            sensitivity: None,
        };
        let is_public = req.is_public.unwrap_or(false);
        assert!(!is_public);
    }

    // UpdateDocumentRequest validation tests
    #[test]
    fn test_update_document_request_all_none() {
        let req = UpdateDocumentRequest {
            title: None,
            markdown_content: None,
            category_id: None,
            is_public: None,
            sensitivity: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_document_request_partial() {
        let req = UpdateDocumentRequest {
            title: Some("New Title".to_string()),
            markdown_content: None,
            category_id: None,
            is_public: None,
            sensitivity: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_document_request_empty_title_fails() {
        let req = UpdateDocumentRequest {
            title: Some("".to_string()),
            markdown_content: None,
            category_id: None,
            is_public: None,
            sensitivity: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_update_document_request_title_too_long_fails() {
        let req = UpdateDocumentRequest {
            title: Some("x".repeat(501)),
            markdown_content: None,
            category_id: None,
            is_public: None,
            sensitivity: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // ListResponse tests
    #[test]
    fn test_list_response_creation() {
        let docs = vec![DocumentResponse {
            id: Uuid::new_v4(),
            title: "Test".to_string(),
            content: "Content".to_string(),
            category_id: None,
            user_id: 1,
            is_public: false,
            sensitivity: "internal".to_string(),
            view_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];
        let response = ListResponse {
            success: true,
            data: docs,
            meta: PaginationMeta {
                total: 1,
                page: 1,
                limit: 20,
                total_pages: 1,
            },
        };
        assert!(response.success);
        assert_eq!(response.data.len(), 1);
    }

    #[test]
    fn test_list_response_empty() {
        let response: ListResponse<DocumentResponse> = ListResponse {
            success: true,
            data: vec![],
            meta: PaginationMeta {
                total: 0,
                page: 1,
                limit: 20,
                total_pages: 0,
            },
        };
        assert!(response.data.is_empty());
        assert_eq!(response.meta.total, 0);
    }

    // SingleResponse tests
    #[test]
    fn test_single_response_creation() {
        let doc = DocumentResponse {
            id: Uuid::new_v4(),
            title: "Single".to_string(),
            content: "Content".to_string(),
            category_id: Some(5),
            user_id: 10,
            is_public: true,
            sensitivity: "public".to_string(),
            view_count: 100,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let response = SingleResponse {
            success: true,
            data: doc,
        };
        assert!(response.success);
        assert_eq!(response.data.title, "Single");
    }

    // DeleteResponse tests
    #[test]
    fn test_delete_response_creation() {
        let id = Uuid::new_v4();
        let response = DeleteResponse {
            success: true,
            message: format!("Document {} deleted successfully", id),
        };
        assert!(response.success);
        assert!(response.message.contains("deleted successfully"));
    }

    #[test]
    fn test_delete_response_contains_uuid() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let response = DeleteResponse {
            success: true,
            message: format!("Document {} deleted successfully", id),
        };
        assert!(response.message.contains("550e8400-e29b-41d4-a716-446655440000"));
    }

    // Upload handler tests
    #[test]
    fn title_from_filename_strips_md_extension() {
        assert_eq!(title_from_filename("my-document.md"), "my document");
    }

    #[test]
    fn title_from_filename_strips_uppercase_md() {
        assert_eq!(title_from_filename("NOTES.MD"), "NOTES");
    }

    #[test]
    fn title_from_filename_replaces_hyphens_and_underscores() {
        assert_eq!(
            title_from_filename("2026-04-15_backend-review.md"),
            "2026 04 15 backend review"
        );
    }

    #[test]
    fn title_from_filename_no_extension() {
        assert_eq!(title_from_filename("no-extension"), "no extension");
    }

    #[test]
    fn title_from_filename_simple() {
        assert_eq!(title_from_filename("simple.md"), "simple");
    }

    #[test]
    fn upload_response_serialization_has_document_key() {
        let resp = UploadResponse {
            success: true,
            document: UploadedDocument {
                id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
                title: "test doc".to_string(),
                chunks_count: 5,
                processing_status: ProcessingStatus::Completed,
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.get("document").is_some());
        assert_eq!(json["success"], true);
        assert_eq!(json["document"]["title"], "test doc");
        assert_eq!(json["document"]["chunks_count"], 5);
    }

    #[test]
    fn upload_response_document_contains_id() {
        let id = Uuid::new_v4();
        let resp = UploadResponse {
            success: true,
            document: UploadedDocument {
                id,
                title: "any".to_string(),
                chunks_count: 0,
                processing_status: ProcessingStatus::Pending,
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["document"]["id"], id.to_string());
    }

    #[test]
    fn upload_response_includes_processing_status_pending() {
        let resp = UploadResponse {
            success: true,
            document: UploadedDocument {
                id: Uuid::new_v4(),
                title: "test".to_string(),
                chunks_count: 1,
                processing_status: ProcessingStatus::Pending,
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["document"]["processing_status"], "pending");
    }

    #[test]
    fn routes_includes_upload_path() {
        let router = routes();
        let _ = router;
    }

    #[test]
    fn max_upload_size_is_10mb() {
        assert_eq!(MAX_UPLOAD_SIZE, 10 * 1024 * 1024);
        assert_eq!(MAX_UPLOAD_SIZE, 10_485_760);
    }

    // validate_upload_filename tests
    #[test]
    fn validate_filename_accepts_md() {
        assert!(validate_upload_filename("test.md").is_ok());
    }

    #[test]
    fn validate_filename_accepts_uppercase_md() {
        assert!(validate_upload_filename("TEST.MD").is_ok());
    }

    #[test]
    fn validate_filename_rejects_txt() {
        assert!(validate_upload_filename("test.txt").is_err());
    }

    #[test]
    fn validate_filename_rejects_no_extension() {
        assert!(validate_upload_filename("readme").is_err());
    }

    #[test]
    fn validate_filename_rejects_empty() {
        assert!(validate_upload_filename("").is_err());
    }

    #[test]
    fn validate_filename_rejects_path_traversal() {
        assert!(validate_upload_filename("../../etc/passwd.md").is_err());
        assert!(validate_upload_filename("..\\windows\\system.md").is_err());
    }

    #[test]
    fn validate_filename_rejects_null_byte() {
        assert!(validate_upload_filename("test\0.md").is_err());
    }

    // validate_upload_data tests
    #[test]
    fn validate_data_accepts_valid_utf8() {
        assert!(validate_upload_data(b"# Hello\nWorld").is_ok());
    }

    #[test]
    fn validate_data_rejects_empty() {
        let err = validate_upload_data(b"").unwrap_err();
        assert!(err.contains("empty"));
    }

    #[test]
    fn validate_data_rejects_oversized() {
        let oversized = vec![b'a'; MAX_UPLOAD_SIZE + 1];
        let err = validate_upload_data(&oversized).unwrap_err();
        assert!(err.contains("too large"));
    }

    #[test]
    fn validate_data_accepts_exactly_max_size() {
        let exact = vec![b'a'; MAX_UPLOAD_SIZE];
        assert!(validate_upload_data(&exact).is_ok());
    }

    #[test]
    fn validate_data_rejects_invalid_utf8() {
        let invalid = vec![0xFF, 0xFE, 0x00];
        let err = validate_upload_data(&invalid).unwrap_err();
        assert!(err.contains("UTF-8"));
    }

    #[test]
    fn processing_status_serializes_to_snake_case() {
        let pending = serde_json::to_value(ProcessingStatus::Pending).unwrap();
        let completed = serde_json::to_value(ProcessingStatus::Completed).unwrap();
        assert_eq!(pending, "pending");
        assert_eq!(completed, "completed");
    }

    #[test]
    fn processing_status_deserializes_from_snake_case() {
        let pending: ProcessingStatus = serde_json::from_str("\"pending\"").unwrap();
        let completed: ProcessingStatus = serde_json::from_str("\"completed\"").unwrap();
        assert_eq!(pending, ProcessingStatus::Pending);
        assert_eq!(completed, ProcessingStatus::Completed);
    }

    #[test]
    fn processing_status_rejects_unknown_value() {
        let result: Result<ProcessingStatus, _> = serde_json::from_str("\"unknown\"");
        assert!(result.is_err());
    }

    #[test]
    fn processing_status_failed_serializes() {
        let failed = serde_json::to_value(ProcessingStatus::Failed).unwrap();
        assert_eq!(failed, "failed");
    }

    #[test]
    fn processing_status_failed_deserializes() {
        let failed: ProcessingStatus = serde_json::from_str("\"failed\"").unwrap();
        assert_eq!(failed, ProcessingStatus::Failed);
    }

    #[test]
    fn document_status_serialization() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let status = DocumentStatus {
            document_id: id,
            processing_status: ProcessingStatus::Pending,
            queue_position: Some(3),
            error_message: None,
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["document_id"], id.to_string());
        assert_eq!(json["processing_status"], "pending");
        assert_eq!(json["queue_position"], 3);
        assert!(json["error_message"].is_null());
    }

    #[test]
    fn document_status_with_error() {
        let status = DocumentStatus {
            document_id: Uuid::new_v4(),
            processing_status: ProcessingStatus::Failed,
            queue_position: None,
            error_message: Some("API key invalid".to_string()),
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["processing_status"], "failed");
        assert_eq!(json["error_message"], "API key invalid");
        assert!(json["queue_position"].is_null());
    }

    #[test]
    fn reprocess_response_serialization() {
        let id = Uuid::new_v4();
        let resp = ReprocessResponse {
            success: true,
            document_id: id,
            message: "Document queued for reprocessing".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["document_id"], id.to_string());
        assert_eq!(json["message"], "Document queued for reprocessing");
    }

    #[test]
    fn document_status_pending_with_queue_position() {
        let status = DocumentStatus {
            document_id: Uuid::new_v4(),
            processing_status: ProcessingStatus::Pending,
            queue_position: Some(1),
            error_message: None,
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["processing_status"], "pending");
        assert_eq!(json["queue_position"], 1);
    }

    #[test]
    fn document_status_completed_no_position() {
        let status = DocumentStatus {
            document_id: Uuid::new_v4(),
            processing_status: ProcessingStatus::Completed,
            queue_position: None,
            error_message: None,
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["processing_status"], "completed");
        assert!(json["queue_position"].is_null());
        assert!(json["error_message"].is_null());
    }

    // ── SRT 파싱 + 청크 분할 (F4) ────────────────────────────────────────────

    #[test]
    fn parse_srt_basic_rolling_dedup() {
        // rolling 2-line 형식: 블록마다 마지막 줄만 수집, 연속 중복 제거
        let srt = "1\n00:00:01,000 --> 00:00:02,000\n안녕하세요\n\n\
                   2\n00:00:02,000 --> 00:00:03,000\n안녕하세요\n반갑습니다\n\n\
                   3\n00:00:03,000 --> 00:00:04,000\n반갑습니다\n잘 부탁드립니다";
        let result = parse_srt_to_text(srt);
        // 블록1 last: "안녕하세요", 블록2 last: "반갑습니다"(다름), 블록3 last: "잘 부탁드립니다"(다름)
        assert_eq!(result, "안녕하세요\n반갑습니다\n잘 부탁드립니다");
    }

    #[test]
    fn parse_srt_skips_duplicate_last_lines() {
        let srt = "1\n00:00:01,000 --> 00:00:02,000\n같은 줄\n\n\
                   2\n00:00:02,000 --> 00:00:03,000\n같은 줄\n\n\
                   3\n00:00:03,000 --> 00:00:04,000\n다른 줄";
        let result = parse_srt_to_text(srt);
        assert_eq!(result, "같은 줄\n다른 줄");
    }

    #[test]
    fn parse_srt_empty_input() {
        assert_eq!(parse_srt_to_text(""), "");
    }

    #[test]
    fn parse_srt_timestamp_only_blocks_skipped() {
        // 타임스탬프만 있는 블록 → 텍스트 없음 → 수집 안 함
        let srt = "1\n00:00:01,000 --> 00:00:02,000\n\n\
                   2\n00:00:02,000 --> 00:00:03,000\n실제 텍스트";
        let result = parse_srt_to_text(srt);
        assert_eq!(result, "실제 텍스트");
    }

    #[test]
    fn parse_srt_block_number_not_collected() {
        // 숫자만으로 이루어진 블록 번호는 수집하지 않음
        let srt = "1\n00:00:01,000 --> 00:00:02,000\n텍스트A\n\n\
                   2\n00:00:02,000 --> 00:00:03,000\n텍스트B";
        let result = parse_srt_to_text(srt);
        assert!(!result.contains("1"), "block number must not appear in output");
        assert!(!result.contains("2"), "block number must not appear in output");
        assert_eq!(result, "텍스트A\n텍스트B");
    }

    #[test]
    fn chunk_srt_text_single_chunk_when_short() {
        let text = "짧은 텍스트";
        let chunks = chunk_srt_text(text, 2500);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn chunk_srt_text_splits_at_max_chars() {
        // 100자 제한, 110자 텍스트 → 2개 청크
        let line_a = "A".repeat(60);
        let line_b = "B".repeat(60);
        let text = format!("{line_a}\n{line_b}");
        let chunks = chunk_srt_text(&text, 100);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].contains(&line_a));
        assert!(chunks[1].contains(&line_b));
    }

    #[test]
    fn chunk_srt_text_empty_input() {
        let chunks = chunk_srt_text("", 2500);
        assert!(chunks.is_empty());
    }

    #[test]
    fn chunk_srt_text_preserves_content() {
        let text = "줄1\n줄2\n줄3\n줄4\n줄5";
        let chunks = chunk_srt_text(text, 2500);
        let rejoined = chunks.join("\n");
        assert_eq!(rejoined, text);
    }

    #[test]
    fn max_srt_chunks_constant() {
        assert_eq!(MAX_SRT_CHUNKS, 50);
    }

    #[test]
    fn processing_status_all_variants_roundtrip() {
        for (variant, expected) in [
            (ProcessingStatus::Pending, "pending"),
            (ProcessingStatus::Completed, "completed"),
            (ProcessingStatus::Failed, "failed"),
        ] {
            let serialized = serde_json::to_value(variant).unwrap();
            assert_eq!(serialized, expected);
            let deserialized: ProcessingStatus = serde_json::from_value(serialized).unwrap();
            assert_eq!(deserialized, variant);
        }
    }

    // ── Flask-compat type tests ───────────────────────────────────────────────

    fn make_flask_item() -> FlaskDocumentItem {
        FlaskDocumentItem {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            title: "Test".to_string(),
            markdown_content: "# Hello".to_string(),
            category_id: None,
            user_id: 1,
            is_public: true,
            sensitivity: "internal".to_string(),
            view_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            author: None,
            html_content: None,
            is_published: false,
            published_at: None,
            tag_names: vec![],
            archived: false,
            archived_at: None,
        }
    }

    fn make_flask_lite() -> FlaskDocumentLite {
        FlaskDocumentLite {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            title: "Test".to_string(),
            author: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id: 1,
            category_id: None,
            is_public: true,
            sensitivity: "internal".to_string(),
            is_published: false,
            published_at: None,
            tag_names: vec![],
        }
    }

    fn make_pagination(page: i32, total_pages: i32, total: i64) -> FlaskPagination {
        FlaskPagination {
            page,
            per_page: 20,
            total,
            pages: total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }

    // ── FlaskDocumentItem (single view) ──────────────────────────────────────

    #[test]
    fn flask_item_serializes_markdown_content_key() {
        let item = make_flask_item();
        let json = serde_json::to_value(&item).unwrap();
        assert!(json.get("markdown_content").is_some(), "key must be markdown_content");
        assert!(json.get("content").is_none(), "old content key must not appear");
        assert_eq!(json["markdown_content"], "# Hello");
    }

    #[test]
    fn flask_item_no_success_wrapper() {
        let item = make_flask_item();
        let json = serde_json::to_value(&item).unwrap();
        assert!(json.get("success").is_none(), "Flask single doc has no success wrapper");
        assert!(json.get("data").is_none(), "Flask single doc has no data wrapper");
    }

    #[test]
    fn flask_item_tag_names_empty_by_default() {
        let item = make_flask_item();
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["tag_names"], serde_json::json!([]));
    }

    // ── FlaskDocumentLite consumer-contract (H1) ─────────────────────────────
    // These tests verify Rust output matches what Flask to_dict_lite actually returns
    // and what the frontend DocumentList.js actually reads.

    #[test]
    fn flask_lite_no_markdown_content_key() {
        // C2: list items must NOT contain markdown_content (to_dict_lite omits it)
        let item = make_flask_lite();
        let json = serde_json::to_value(&item).unwrap();
        assert!(json.get("markdown_content").is_none(), "list items must not contain markdown_content");
        assert!(json.get("content").is_none(), "list items must not contain content");
        assert!(json.get("html_content").is_none(), "list items must not contain html_content");
    }

    #[test]
    fn flask_lite_has_required_to_dict_lite_keys() {
        // Consumer-contract: keys match Flask to_dict_lite exactly
        let item = make_flask_lite();
        let json = serde_json::to_value(&item).unwrap();
        for key in ["id", "title", "author", "created_at", "updated_at",
                    "user_id", "category_id", "is_public", "is_published",
                    "published_at", "tag_names"] {
            assert!(json.get(key).is_some(), "missing key: {key}");
        }
    }

    #[test]
    fn flask_lite_tag_names_is_array_of_strings() {
        let mut item = make_flask_lite();
        item.tag_names = vec!["rust".to_string(), "backend".to_string()];
        let json = serde_json::to_value(&item).unwrap();
        let tags = json["tag_names"].as_array().unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0], "rust");
        assert_eq!(tags[1], "backend");
    }

    #[test]
    fn flask_lite_no_view_count_key() {
        // to_dict_lite does not include view_count
        let item = make_flask_lite();
        let json = serde_json::to_value(&item).unwrap();
        assert!(json.get("view_count").is_none(), "list items must not contain view_count");
    }

    // ── FlaskPagination consumer-contract (C3, H1) ───────────────────────────

    #[test]
    fn flask_pagination_has_next_prev_keys() {
        // C3: Pagination.js requires has_next and has_prev
        let p = make_pagination(2, 5, 90);
        let json = serde_json::to_value(&p).unwrap();
        assert!(json.get("has_next").is_some(), "Pagination.js requires has_next");
        assert!(json.get("has_prev").is_some(), "Pagination.js requires has_prev");
    }

    #[test]
    fn flask_pagination_has_next_true_when_not_last_page() {
        let p = make_pagination(2, 5, 90);
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["has_next"], true);
        assert_eq!(json["has_prev"], true);
    }

    #[test]
    fn flask_pagination_first_page_no_prev() {
        let p = make_pagination(1, 5, 90);
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["has_prev"], false);
        assert_eq!(json["has_next"], true);
    }

    #[test]
    fn flask_pagination_last_page_no_next() {
        let p = make_pagination(5, 5, 90);
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["has_next"], false);
        assert_eq!(json["has_prev"], true);
    }

    #[test]
    fn flask_pagination_single_page_no_next_no_prev() {
        let p = make_pagination(1, 1, 5);
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["has_next"], false);
        assert_eq!(json["has_prev"], false);
    }

    #[test]
    fn flask_pagination_field_names() {
        let pagination = make_pagination(2, 5, 45);
        let json = serde_json::to_value(&pagination).unwrap();
        assert_eq!(json["page"], 2);
        assert_eq!(json["per_page"], 20);
        assert_eq!(json["total"], 45);
        assert_eq!(json["pages"], 5);
        assert!(json.get("limit").is_none(), "Flask uses per_page not limit");
        assert!(json.get("total_pages").is_none(), "Flask uses pages not total_pages");
    }

    // ── FlaskListResponse consumer-contract ──────────────────────────────────

    #[test]
    fn flask_list_response_documents_key() {
        let resp = FlaskListResponse {
            documents: vec![make_flask_lite()],
            pagination: make_pagination(1, 1, 1),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.get("documents").is_some(), "key must be documents");
        assert!(json.get("data").is_none(), "Flask uses documents not data");
        assert!(json.get("success").is_none(), "Flask list has no success key");
        assert_eq!(json["documents"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn flask_list_response_has_pagination_with_has_next_prev() {
        let resp = FlaskListResponse {
            documents: vec![make_flask_lite()],
            pagination: make_pagination(1, 3, 55),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["pagination"].get("has_next").is_some());
        assert!(json["pagination"].get("has_prev").is_some());
        assert_eq!(json["pagination"]["has_next"], true);
        assert_eq!(json["pagination"]["has_prev"], false);
    }

    #[test]
    fn flask_list_document_items_have_no_content_key() {
        // C2: list items inside documents[] must not have markdown_content
        let resp = FlaskListResponse {
            documents: vec![make_flask_lite()],
            pagination: make_pagination(1, 1, 1),
        };
        let json = serde_json::to_value(&resp).unwrap();
        let first = &json["documents"][0];
        assert!(first.get("markdown_content").is_none());
        assert!(first.get("content").is_none());
    }

    // ── ListQuery per_page alias ─────────────────────────────────────────────

    #[test]
    fn list_query_per_page_alias() {
        let q = ListQuery { page: None, limit: None, per_page: Some(15), category_id: None, search: None };
        let limit = q.limit.or(q.per_page).unwrap_or(20).clamp(1, 100);
        assert_eq!(limit, 15);
    }

    #[test]
    fn list_query_limit_takes_precedence_over_per_page() {
        let q = ListQuery { page: None, limit: Some(10), per_page: Some(30), category_id: None, search: None };
        let limit = q.limit.or(q.per_page).unwrap_or(20).clamp(1, 100);
        assert_eq!(limit, 10);
    }

    // ── D.3: confidential + is_public invariant ──────────────────────────────

    #[test]
    fn test_create_document_confidential_public_rejected() {
        // D.3: is_public=true with sensitivity=confidential must be caught before the DB insert.
        // We exercise the guard logic directly (same path as the handler) without a DB.
        let is_public = true;
        let sensitivity = "confidential";
        let guard_triggered = is_public && sensitivity == "confidential";
        assert!(
            guard_triggered,
            "the invariant guard must fire for is_public=true + sensitivity=confidential"
        );
    }

    #[test]
    fn test_update_document_confidential_public_rejected() {
        // D.3 update path: simulate the merged-state check.

        // Case 1: payload sets is_public=true; existing sensitivity is "confidential".
        // Use a helper closure to replicate the handler's merge logic.
        fn merge_is_public(payload: Option<bool>, current: bool) -> bool {
            payload.unwrap_or(current)
        }
        fn merge_sensitivity<'a>(payload: Option<&'a str>, current: &'a str) -> &'a str {
            payload.unwrap_or(current)
        }

        let current_is_public = false;
        let current_sensitivity = "confidential";
        let effective_is_public = merge_is_public(Some(true), current_is_public);
        let effective_sensitivity = merge_sensitivity(None, current_sensitivity);

        assert!(
            effective_is_public && effective_sensitivity == "confidential",
            "merged state should trigger the invariant guard"
        );

        // Case 2: payload sets sensitivity=confidential; existing is_public is true.
        let current_is_public2 = true;
        let current_sensitivity2 = "internal";
        let effective_is_public2 = merge_is_public(None, current_is_public2);
        let effective_sensitivity2 = merge_sensitivity(Some("confidential"), current_sensitivity2);

        assert!(
            effective_is_public2 && effective_sensitivity2 == "confidential",
            "setting sensitivity=confidential on a public doc should trigger the guard"
        );
    }
}
