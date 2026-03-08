use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{
        ExportFormat, ExportJob, ExportRequest, ExportTheme, FunctionContext, FunctionSummary,
        ImportJob, ImportRequest, ThemeSummary,
    },
    services::{export::HtmlRenderer, export::SlidesWriter, ExportService, FunctionRegistry},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        // Existing export routes
        .route("/", post(start_export))
        .route("/download", get(download_export))
        .route("/status/{id}", get(get_export_status))
        .route("/import", post(start_import))
        // New format-specific export routes
        .route("/html", post(export_html))
        .route("/slides", post(export_slides))
        .route("/preview", post(preview_with_functions))
        // Theme management
        .route("/themes", get(list_themes))
        .route("/themes/{name}", get(get_theme))
        // Function management
        .route("/functions", get(list_functions))
        .route("/functions/{name}", get(get_function))
}

#[derive(Debug, Serialize)]
pub struct ExportJobResponse {
    pub success: bool,
    pub data: ExportJob,
}

async fn start_export(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<ExportRequest>,
) -> AppResult<Json<ExportJobResponse>> {
    let service = ExportService::new(state.db.clone());
    let job = service.start_export(auth_user.id, payload).await?;

    Ok(Json(ExportJobResponse {
        success: true,
        data: job,
    }))
}

#[derive(Debug, Deserialize)]
pub struct DownloadQuery {
    pub format: Option<ExportFormat>,
    pub document_ids: Option<String>,
    pub category_id: Option<i32>,
}

async fn download_export(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<DownloadQuery>,
) -> impl IntoResponse {
    let service = ExportService::new(state.db.clone());

    let request = ExportRequest {
        document_ids: query.document_ids.map(|s| {
            s.split(',')
                .filter_map(|id| uuid::Uuid::parse_str(id.trim()).ok())
                .collect()
        }),
        category_id: query.category_id,
        format: query.format.clone(),
        include_metadata: Some(true),
        include_attachments: Some(false),
        include_comments: Some(false),
        include_versions: Some(false),
    };

    let documents = match service.export_documents(auth_user.id, &request).await {
        Ok(docs) => docs,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Export failed: {}", e),
            )
                .into_response();
        }
    };

    let format = query.format.unwrap_or_default();
    let (content, content_type, extension) = match format {
        ExportFormat::Json => {
            let json = service.to_json(&documents).unwrap_or_default();
            (json, "application/json", "json")
        }
        ExportFormat::Csv => {
            let csv = service.to_csv(&documents).unwrap_or_default();
            (csv, "text/csv", "csv")
        }
        ExportFormat::Markdown => {
            let md = service.to_markdown(&documents).unwrap_or_default();
            (md, "text/markdown", "md")
        }
        _ => {
            let json = service.to_json(&documents).unwrap_or_default();
            (json, "application/json", "json")
        }
    };

    let filename = format!(
        "export_{}.{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S"),
        extension
    );

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", filename)
            .parse()
            .unwrap(),
    );

    (headers, content).into_response()
}

#[derive(Debug, Serialize)]
pub struct ExportStatusResponse {
    pub success: bool,
    pub data: Option<ExportJob>,
}

async fn get_export_status(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<ExportStatusResponse>> {
    let service = ExportService::new(state.db.clone());
    let job = service.get_export_status(&id).await?;

    Ok(Json(ExportStatusResponse {
        success: true,
        data: job,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ImportPayload {
    pub content: String,
    #[serde(flatten)]
    pub request: ImportRequest,
}

#[derive(Debug, Serialize)]
pub struct ImportJobResponse {
    pub success: bool,
    pub data: ImportJob,
}

async fn start_import(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<ImportPayload>,
) -> AppResult<Json<ImportJobResponse>> {
    let service = ExportService::new(state.db.clone());
    let job = service
        .import_from_json(auth_user.id, &payload.content, payload.request.category_id)
        .await?;

    Ok(Json(ImportJobResponse {
        success: true,
        data: job,
    }))
}

// -------------------------------------------------------------------------
// HTML Export
// -------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct HtmlExportRequest {
    pub content: String,
    pub title: Option<String>,
    pub theme: Option<String>,
}

async fn export_html(
    _auth_user: AuthUser,
    Json(payload): Json<HtmlExportRequest>,
) -> impl IntoResponse {
    let theme = payload
        .theme
        .as_deref()
        .and_then(ExportTheme::by_name)
        .unwrap_or_else(ExportTheme::light);

    let renderer = HtmlRenderer::new(theme);

    let html = if let Some(title) = payload.title {
        renderer.render_with_title(&payload.content, &title)
    } else {
        renderer.render(&payload.content)
    };

    match html {
        Ok(html) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "text/html; charset=utf-8".parse().unwrap());
            (headers, html).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("HTML generation failed: {}", e),
        )
            .into_response(),
    }
}

// -------------------------------------------------------------------------
// Slides Export
// -------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SlidesExportRequest {
    pub content: String,
    pub title: Option<String>,
    pub theme: Option<String>,
    pub reveal_theme: Option<String>,
}

async fn export_slides(
    _auth_user: AuthUser,
    Json(payload): Json<SlidesExportRequest>,
) -> impl IntoResponse {
    let export_theme = payload
        .theme
        .as_deref()
        .and_then(ExportTheme::by_name)
        .unwrap_or_else(ExportTheme::light);

    let mut config = crate::services::export::RevealConfig::default();
    if let Some(reveal_theme) = &payload.reveal_theme {
        config.theme = reveal_theme.clone();
    }

    let writer = SlidesWriter::new(export_theme, config);

    let html = if let Some(title) = payload.title {
        writer.render_with_title(&payload.content, &title)
    } else {
        writer.render(&payload.content)
    };

    match html {
        Ok(html) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "text/html; charset=utf-8".parse().unwrap());
            (headers, html).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Slides generation failed: {}", e),
        )
            .into_response(),
    }
}

// -------------------------------------------------------------------------
// Preview with Function Expansion
// -------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct PreviewRequest {
    pub content: String,
    pub variables: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
pub struct PreviewResponse {
    pub success: bool,
    pub content: String,
    pub expanded_functions: Vec<String>,
}

async fn preview_with_functions(
    _auth_user: AuthUser,
    Json(payload): Json<PreviewRequest>,
) -> AppResult<Json<PreviewResponse>> {
    let registry = FunctionRegistry::new();

    let ctx = FunctionContext {
        variables: payload.variables.unwrap_or_default(),
        document_id: None,
        base_path: None,
    };

    let expanded = registry.expand_document(&payload.content, &ctx);

    // Get list of functions used
    let parser = crate::services::FunctionParser::new();
    let calls = parser.parse(&payload.content);
    let expanded_functions: Vec<String> = calls.iter().map(|c| c.name.clone()).collect();

    Ok(Json(PreviewResponse {
        success: true,
        content: expanded,
        expanded_functions,
    }))
}

// -------------------------------------------------------------------------
// Theme Management
// -------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ThemesListResponse {
    pub success: bool,
    pub themes: Vec<ThemeSummary>,
}

async fn list_themes(_auth_user: AuthUser) -> AppResult<Json<ThemesListResponse>> {
    let themes: Vec<ThemeSummary> = ExportTheme::builtin_themes()
        .iter()
        .map(ThemeSummary::from)
        .collect();

    Ok(Json(ThemesListResponse {
        success: true,
        themes,
    }))
}

#[derive(Debug, Serialize)]
pub struct ThemeResponse {
    pub success: bool,
    pub theme: Option<ExportTheme>,
}

async fn get_theme(
    _auth_user: AuthUser,
    Path(name): Path<String>,
) -> AppResult<Json<ThemeResponse>> {
    let theme = ExportTheme::by_name(&name);

    Ok(Json(ThemeResponse {
        success: true,
        theme,
    }))
}

// -------------------------------------------------------------------------
// Function Management
// -------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct FunctionsListResponse {
    pub success: bool,
    pub functions: Vec<FunctionSummary>,
}

async fn list_functions(_auth_user: AuthUser) -> AppResult<Json<FunctionsListResponse>> {
    let registry = FunctionRegistry::new();
    let functions: Vec<FunctionSummary> = registry.definitions().iter().map(|d| (*d).into()).collect();

    Ok(Json(FunctionsListResponse {
        success: true,
        functions,
    }))
}

#[derive(Debug, Serialize)]
pub struct FunctionResponse {
    pub success: bool,
    pub function: Option<crate::models::FunctionDefinition>,
}

async fn get_function(
    _auth_user: AuthUser,
    Path(name): Path<String>,
) -> AppResult<Json<FunctionResponse>> {
    let registry = FunctionRegistry::new();
    let function = registry.get(&name).cloned();

    Ok(Json(FunctionResponse {
        success: true,
        function,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ExportStatus;

    // -------------------------------------------------------------------------
    // DownloadQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_download_query_deserialization() {
        let json = r#"{"format": "json", "category_id": 5}"#;
        let query: DownloadQuery = serde_json::from_str(json).unwrap();
        assert!(matches!(query.format, Some(ExportFormat::Json)));
        assert_eq!(query.category_id, Some(5));
    }

    #[test]
    fn test_download_query_csv_format() {
        let json = r#"{"format": "csv"}"#;
        let query: DownloadQuery = serde_json::from_str(json).unwrap();
        assert!(matches!(query.format, Some(ExportFormat::Csv)));
    }

    #[test]
    fn test_download_query_markdown_format() {
        let json = r#"{"format": "markdown"}"#;
        let query: DownloadQuery = serde_json::from_str(json).unwrap();
        assert!(matches!(query.format, Some(ExportFormat::Markdown)));
    }

    #[test]
    fn test_download_query_with_document_ids() {
        let json = r#"{"document_ids": "uuid1,uuid2,uuid3"}"#;
        let query: DownloadQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.document_ids, Some("uuid1,uuid2,uuid3".to_string()));
    }

    #[test]
    fn test_download_query_empty() {
        let json = r#"{}"#;
        let query: DownloadQuery = serde_json::from_str(json).unwrap();
        assert!(query.format.is_none());
        assert!(query.document_ids.is_none());
        assert!(query.category_id.is_none());
    }

    // -------------------------------------------------------------------------
    // ExportJobResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_export_job_response_serialization() {
        let now = chrono::Utc::now();
        let response = ExportJobResponse {
            success: true,
            data: ExportJob {
                id: "export-001".to_string(),
                user_id: 1,
                format: ExportFormat::Json,
                status: ExportStatus::Pending,
                document_count: 10,
                progress_percent: 0,
                download_url: None,
                error_message: None,
                created_at: now,
                completed_at: None,
                expires_at: None,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"id\":\"export-001\""));
        assert!(json.contains("\"document_count\":10"));
    }

    #[test]
    fn test_export_job_response_completed() {
        let now = chrono::Utc::now();
        let response = ExportJobResponse {
            success: true,
            data: ExportJob {
                id: "export-002".to_string(),
                user_id: 1,
                format: ExportFormat::Csv,
                status: ExportStatus::Completed,
                document_count: 50,
                progress_percent: 100,
                download_url: Some("/downloads/export-002.csv".to_string()),
                error_message: None,
                created_at: now,
                completed_at: Some(now),
                expires_at: Some(now + chrono::Duration::days(7)),
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"progress_percent\":100"));
        assert!(json.contains("\"download_url\""));
    }

    // -------------------------------------------------------------------------
    // ExportStatusResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_export_status_response_found() {
        let now = chrono::Utc::now();
        let response = ExportStatusResponse {
            success: true,
            data: Some(ExportJob {
                id: "export-003".to_string(),
                user_id: 1,
                format: ExportFormat::Markdown,
                status: ExportStatus::Processing,
                document_count: 25,
                progress_percent: 50,
                download_url: None,
                error_message: None,
                created_at: now,
                completed_at: None,
                expires_at: None,
            }),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"progress_percent\":50"));
    }

    #[test]
    fn test_export_status_response_not_found() {
        let response = ExportStatusResponse {
            success: true,
            data: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":null"));
    }

    // -------------------------------------------------------------------------
    // ImportPayload tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_import_payload_deserialization() {
        let json = r#"{"content": "[{\"title\": \"Doc\"}]", "format": "json", "category_id": 1}"#;
        let payload: ImportPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.content, "[{\"title\": \"Doc\"}]");
        assert_eq!(payload.request.category_id, Some(1));
    }

    #[test]
    fn test_import_payload_minimal() {
        let json = r#"{"content": "[]"}"#;
        let payload: ImportPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.content, "[]");
    }

    // -------------------------------------------------------------------------
    // ImportJobResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_import_job_response_serialization() {
        let now = chrono::Utc::now();
        let response = ImportJobResponse {
            success: true,
            data: ImportJob {
                id: "import-001".to_string(),
                user_id: 1,
                status: ExportStatus::Completed,
                total_items: 10,
                processed_items: 10,
                success_count: 9,
                error_count: 1,
                errors: vec![],
                created_at: now,
                completed_at: Some(now),
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"total_items\":10"));
        assert!(json.contains("\"success_count\":9"));
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
