use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Serialize;

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{
        ApplyTemplateRequest, CreateTemplate, Template, TemplatePreview, TemplateQuery,
        UpdateTemplate,
    },
    services::TemplateService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_templates))
        .route("/", post(create_template))
        .route("/{id}", get(get_template))
        .route("/{id}", put(update_template))
        .route("/{id}", delete(delete_template))
        .route("/{id}/preview", post(preview_template))
        .route("/{id}/apply", post(apply_template))
}

#[derive(Debug, Serialize)]
pub struct TemplatesResponse {
    pub success: bool,
    pub data: Vec<Template>,
}

async fn list_templates(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<TemplateQuery>,
) -> AppResult<Json<TemplatesResponse>> {
    let service = TemplateService::new(state.db.clone());
    let templates = service.list_templates(auth_user.id, query).await?;

    Ok(Json(TemplatesResponse {
        success: true,
        data: templates,
    }))
}

#[derive(Debug, Serialize)]
pub struct TemplateResponse {
    pub success: bool,
    pub data: Template,
}

async fn get_template(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<TemplateResponse>> {
    let service = TemplateService::new(state.db.clone());
    let template = service
        .get_template(auth_user.id, id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Template not found".to_string()))?;

    Ok(Json(TemplateResponse {
        success: true,
        data: template,
    }))
}

async fn create_template(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateTemplate>,
) -> AppResult<Json<TemplateResponse>> {
    let service = TemplateService::new(state.db.clone());
    let template = service.create_template(auth_user.id, payload).await?;

    Ok(Json(TemplateResponse {
        success: true,
        data: template,
    }))
}

async fn update_template(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateTemplate>,
) -> AppResult<Json<TemplateResponse>> {
    let service = TemplateService::new(state.db.clone());
    let template = service.update_template(auth_user.id, id, payload).await?;

    Ok(Json(TemplateResponse {
        success: true,
        data: template,
    }))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_template(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<DeleteResponse>> {
    let service = TemplateService::new(state.db.clone());
    service.delete_template(auth_user.id, id).await?;

    Ok(Json(DeleteResponse {
        success: true,
        message: "Template deleted".to_string(),
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct PreviewRequest {
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct PreviewResponse {
    pub success: bool,
    pub data: TemplatePreview,
}

async fn preview_template(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<PreviewRequest>,
) -> AppResult<Json<PreviewResponse>> {
    let service = TemplateService::new(state.db.clone());
    let template = service
        .get_template(auth_user.id, id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Template not found".to_string()))?;

    let preview = service.preview_template(&template, payload.variables.as_ref())?;

    Ok(Json(PreviewResponse {
        success: true,
        data: preview,
    }))
}

#[derive(Debug, Serialize)]
pub struct ApplyResponse {
    pub success: bool,
    pub document_id: uuid::Uuid,
}

async fn apply_template(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(mut payload): Json<ApplyTemplateRequest>,
) -> AppResult<Json<ApplyResponse>> {
    payload.template_id = id;

    let service = TemplateService::new(state.db.clone());
    let document_id = service.apply_template(auth_user.id, payload).await?;

    Ok(Json(ApplyResponse {
        success: true,
        document_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{TemplateVariable, VariableType};

    // -------------------------------------------------------------------------
    // PreviewRequest tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_preview_request_deserialization() {
        let json = r#"{"variables": {"name": "Test", "count": 5}}"#;
        let request: PreviewRequest = serde_json::from_str(json).unwrap();
        assert!(request.variables.is_some());
    }

    #[test]
    fn test_preview_request_empty() {
        let json = r#"{}"#;
        let request: PreviewRequest = serde_json::from_str(json).unwrap();
        assert!(request.variables.is_none());
    }

    #[test]
    fn test_preview_request_with_null_variables() {
        let json = r#"{"variables": null}"#;
        let request: PreviewRequest = serde_json::from_str(json).unwrap();
        assert!(request.variables.is_none());
    }

    // -------------------------------------------------------------------------
    // TemplatesResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_templates_response_serialization() {
        let response = TemplatesResponse {
            success: true,
            data: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":[]"));
    }

    #[test]
    fn test_templates_response_with_templates() {
        let now = chrono::Utc::now();
        let response = TemplatesResponse {
            success: true,
            data: vec![Template {
                id: 1,
                name: "Meeting Notes".to_string(),
                description: Some("Template for meeting notes".to_string()),
                content: "# {{title}}\n\nDate: {{date}}\n\n## Attendees\n{{attendees}}".to_string(),
                category_id: Some(5),
                category_name: Some("Work".to_string()),
                variables: vec![TemplateVariable {
                    name: "title".to_string(),
                    description: Some("Meeting title".to_string()),
                    default_value: None,
                    required: true,
                    var_type: VariableType::Text,
                }],
                tags: vec!["meeting".to_string(), "notes".to_string()],
                is_public: true,
                usage_count: 42,
                created_by: 1,
                created_by_name: Some("admin".to_string()),
                created_at: now,
                updated_at: now,
            }],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"Meeting Notes\""));
        assert!(json.contains("\"usage_count\":42"));
    }

    // -------------------------------------------------------------------------
    // TemplateResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_template_response_serialization() {
        let now = chrono::Utc::now();
        let response = TemplateResponse {
            success: true,
            data: Template {
                id: 1,
                name: "Bug Report".to_string(),
                description: None,
                content: "## Bug Report\n\n{{description}}".to_string(),
                category_id: None,
                category_name: None,
                variables: vec![],
                tags: vec![],
                is_public: false,
                usage_count: 0,
                created_by: 1,
                created_by_name: None,
                created_at: now,
                updated_at: now,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"name\":\"Bug Report\""));
        assert!(json.contains("\"is_public\":false"));
    }

    // -------------------------------------------------------------------------
    // DeleteResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_delete_response_serialization() {
        let response = DeleteResponse {
            success: true,
            message: "Template deleted".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"Template deleted\""));
    }

    // -------------------------------------------------------------------------
    // PreviewResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_preview_response_serialization() {
        let response = PreviewResponse {
            success: true,
            data: TemplatePreview {
                content: "# Meeting Notes\n\nDate: 2026-03-08".to_string(),
                title: Some("Meeting Notes".to_string()),
                missing_variables: vec![],
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"title\":\"Meeting Notes\""));
    }

    #[test]
    fn test_preview_response_with_missing_variables() {
        let response = PreviewResponse {
            success: true,
            data: TemplatePreview {
                content: "# {{title}}\n\nDate: {{date}}".to_string(),
                title: None,
                missing_variables: vec!["title".to_string(), "date".to_string()],
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"missing_variables\":[\"title\",\"date\"]"));
    }

    // -------------------------------------------------------------------------
    // ApplyResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_apply_response_serialization() {
        let doc_id = uuid::Uuid::new_v4();
        let response = ApplyResponse {
            success: true,
            document_id: doc_id,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains(&doc_id.to_string()));
    }

    // -------------------------------------------------------------------------
    // TemplateQuery tests (from models but used here)
    // -------------------------------------------------------------------------

    #[test]
    fn test_template_query_deserialization() {
        let json = r#"{"page": 2, "limit": 20, "category_id": 5, "is_public": true}"#;
        let query: TemplateQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page, Some(2));
        assert_eq!(query.limit, Some(20));
        assert_eq!(query.category_id, Some(5));
        assert_eq!(query.is_public, Some(true));
    }

    #[test]
    fn test_template_query_with_search() {
        let json = r#"{"search": "meeting"}"#;
        let query: TemplateQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.search, Some("meeting".to_string()));
    }

    #[test]
    fn test_template_query_empty() {
        let json = r#"{}"#;
        let query: TemplateQuery = serde_json::from_str(json).unwrap();
        assert!(query.page.is_none());
        assert!(query.limit.is_none());
        assert!(query.search.is_none());
    }

    // -------------------------------------------------------------------------
    // CreateTemplate tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_create_template_deserialization() {
        let json = r#"{"name": "New Template", "content": "Content with {{variable}}"}"#;
        let create: CreateTemplate = serde_json::from_str(json).unwrap();
        assert_eq!(create.name, "New Template");
        assert!(create.content.contains("{{variable}}"));
    }

    #[test]
    fn test_create_template_full() {
        let json = r#"{
            "name": "Full Template",
            "description": "A complete template",
            "content": "Content here",
            "category_id": 1,
            "variables": [{"name": "title", "required": true, "var_type": "text"}],
            "tags": ["tag1", "tag2"],
            "is_public": true
        }"#;
        let create: CreateTemplate = serde_json::from_str(json).unwrap();
        assert_eq!(create.name, "Full Template");
        assert_eq!(create.is_public, Some(true));
        assert!(create.variables.is_some());
    }

    // -------------------------------------------------------------------------
    // UpdateTemplate tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_update_template_partial() {
        let json = r#"{"name": "Updated Name"}"#;
        let update: UpdateTemplate = serde_json::from_str(json).unwrap();
        assert_eq!(update.name, Some("Updated Name".to_string()));
        assert!(update.content.is_none());
    }

    #[test]
    fn test_update_template_full() {
        let json = r#"{
            "name": "Updated",
            "description": "New description",
            "content": "New content",
            "is_public": false
        }"#;
        let update: UpdateTemplate = serde_json::from_str(json).unwrap();
        assert_eq!(update.name, Some("Updated".to_string()));
        assert_eq!(update.is_public, Some(false));
    }

    // -------------------------------------------------------------------------
    // ApplyTemplateRequest tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_apply_template_request_deserialization() {
        let json = r#"{"template_id": 1, "title": "My Document"}"#;
        let request: ApplyTemplateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.template_id, 1);
        assert_eq!(request.title, Some("My Document".to_string()));
    }

    #[test]
    fn test_apply_template_request_with_variables() {
        let json = r#"{"template_id": 5, "variables": {"name": "Test", "date": "2026-03-08"}}"#;
        let request: ApplyTemplateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.template_id, 5);
        assert!(request.variables.is_some());
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
