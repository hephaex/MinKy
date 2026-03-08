use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{EmbeddingResponse, SuggestionRequest, SuggestionResponse, SuggestionType},
    services::AIService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/suggest", post(generate_suggestion))
        .route("/suggest/title", post(suggest_title))
        .route("/suggest/summary", post(suggest_summary))
        .route("/suggest/tags", post(suggest_tags))
        .route("/improve", post(improve_text))
        .route("/embedding", post(generate_embedding))
}

#[derive(Debug, Deserialize, Validate)]
pub struct SuggestionRequestBody {
    #[validate(length(min = 1, max = 50000))]
    pub content: String,
    pub suggestion_type: SuggestionType,
    pub context: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestionResponseBody {
    pub success: bool,
    pub data: SuggestionResponse,
}

async fn generate_suggestion(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<SuggestionRequestBody>,
) -> AppResult<Json<SuggestionResponseBody>> {
    let service = AIService::new(state.config.clone());

    let response = service
        .generate_suggestion(SuggestionRequest {
            content: payload.content,
            suggestion_type: payload.suggestion_type,
            context: payload.context,
        })
        .await?;

    Ok(Json(SuggestionResponseBody {
        success: true,
        data: response,
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct ContentRequest {
    #[validate(length(min = 1, max = 50000))]
    pub content: String,
    pub context: Option<String>,
}

async fn suggest_title(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<ContentRequest>,
) -> AppResult<Json<SuggestionResponseBody>> {
    let service = AIService::new(state.config.clone());

    let response = service
        .generate_suggestion(SuggestionRequest {
            content: payload.content,
            suggestion_type: SuggestionType::Title,
            context: payload.context,
        })
        .await?;

    Ok(Json(SuggestionResponseBody {
        success: true,
        data: response,
    }))
}

async fn suggest_summary(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<ContentRequest>,
) -> AppResult<Json<SuggestionResponseBody>> {
    let service = AIService::new(state.config.clone());

    let response = service
        .generate_suggestion(SuggestionRequest {
            content: payload.content,
            suggestion_type: SuggestionType::Summary,
            context: payload.context,
        })
        .await?;

    Ok(Json(SuggestionResponseBody {
        success: true,
        data: response,
    }))
}

async fn suggest_tags(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<ContentRequest>,
) -> AppResult<Json<SuggestionResponseBody>> {
    let service = AIService::new(state.config.clone());

    let response = service
        .generate_suggestion(SuggestionRequest {
            content: payload.content,
            suggestion_type: SuggestionType::Tags,
            context: payload.context,
        })
        .await?;

    Ok(Json(SuggestionResponseBody {
        success: true,
        data: response,
    }))
}

async fn improve_text(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<ContentRequest>,
) -> AppResult<Json<SuggestionResponseBody>> {
    let service = AIService::new(state.config.clone());

    let response = service
        .generate_suggestion(SuggestionRequest {
            content: payload.content,
            suggestion_type: SuggestionType::Improve,
            context: payload.context,
        })
        .await?;

    Ok(Json(SuggestionResponseBody {
        success: true,
        data: response,
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct EmbeddingRequest {
    #[validate(length(min = 1, max = 10000))]
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingResponseBody {
    pub success: bool,
    pub data: EmbeddingResponse,
}

async fn generate_embedding(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<EmbeddingRequest>,
) -> AppResult<Json<EmbeddingResponseBody>> {
    let service = AIService::new(state.config.clone());

    let response = service.generate_embedding(&payload.text).await?;

    Ok(Json(EmbeddingResponseBody {
        success: true,
        data: response,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // SuggestionRequestBody tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_suggestion_request_body_deserialization() {
        let json = r#"{"content": "This is test content", "suggestion_type": "title"}"#;
        let request: SuggestionRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "This is test content");
        assert!(matches!(request.suggestion_type, SuggestionType::Title));
        assert!(request.context.is_none());
    }

    #[test]
    fn test_suggestion_request_body_with_context() {
        let json = r#"{"content": "Content", "suggestion_type": "summary", "context": "Tech blog"}"#;
        let request: SuggestionRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(request.context, Some("Tech blog".to_string()));
    }

    #[test]
    fn test_suggestion_request_body_all_types() {
        let types = vec![
            ("title", SuggestionType::Title),
            ("summary", SuggestionType::Summary),
            ("tags", SuggestionType::Tags),
            ("improve", SuggestionType::Improve),
        ];

        for (type_str, expected_type) in types {
            let json = format!(r#"{{"content": "test", "suggestion_type": "{}"}}"#, type_str);
            let request: SuggestionRequestBody = serde_json::from_str(&json).unwrap();
            assert!(
                std::mem::discriminant(&request.suggestion_type) == std::mem::discriminant(&expected_type),
                "Failed for type: {}", type_str
            );
        }
    }

    // -------------------------------------------------------------------------
    // ContentRequest tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_content_request_deserialization() {
        let json = r#"{"content": "Some content here"}"#;
        let request: ContentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "Some content here");
        assert!(request.context.is_none());
    }

    #[test]
    fn test_content_request_with_context() {
        let json = r#"{"content": "Content", "context": "Additional context"}"#;
        let request: ContentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.context, Some("Additional context".to_string()));
    }

    // -------------------------------------------------------------------------
    // EmbeddingRequest tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_embedding_request_deserialization() {
        let json = r#"{"text": "Text to embed"}"#;
        let request: EmbeddingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.text, "Text to embed");
    }

    // -------------------------------------------------------------------------
    // SuggestionResponseBody tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_suggestion_response_body_serialization() {
        let response = SuggestionResponseBody {
            success: true,
            data: SuggestionResponse {
                suggestion: "Generated Title".to_string(),
                suggestion_type: SuggestionType::Title,
                tokens_used: 50,
                model: "claude-3-haiku".to_string(),
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"suggestion\":\"Generated Title\""));
        assert!(json.contains("\"tokens_used\":50"));
    }

    #[test]
    fn test_suggestion_response_all_types() {
        let types = vec![
            SuggestionType::Title,
            SuggestionType::Summary,
            SuggestionType::Tags,
            SuggestionType::Improve,
        ];

        for suggestion_type in types {
            let response = SuggestionResponseBody {
                success: true,
                data: SuggestionResponse {
                    suggestion: "Test".to_string(),
                    suggestion_type: suggestion_type.clone(),
                    tokens_used: 10,
                    model: "model".to_string(),
                },
            };
            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"success\":true"));
        }
    }

    // -------------------------------------------------------------------------
    // EmbeddingResponseBody tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_embedding_response_body_serialization() {
        let response = EmbeddingResponseBody {
            success: true,
            data: EmbeddingResponse {
                embedding: vec![0.1, 0.2, 0.3, 0.4, 0.5],
                dimensions: 5,
                model: "text-embedding-3-small".to_string(),
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"dimensions\":5"));
        assert!(json.contains("\"model\":\"text-embedding-3-small\""));
    }

    #[test]
    fn test_embedding_response_body_large_dimensions() {
        let embedding = vec![0.0f32; 1536];
        let response = EmbeddingResponseBody {
            success: true,
            data: EmbeddingResponse {
                embedding,
                dimensions: 1536,
                model: "text-embedding-3-small".to_string(),
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"dimensions\":1536"));
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
