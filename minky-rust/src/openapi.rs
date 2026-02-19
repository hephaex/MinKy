//! OpenAPI 3.0 specification endpoint for MinKy API.
//!
//! Serves a static OpenAPI JSON at `GET /api/docs/openapi.json` and
//! a Swagger UI redirect page at `GET /api/docs/`.
//!
//! The spec is hand-maintained here.  For a live project, consider using
//! utoipa macros on each handler, but the static approach avoids invasive
//! changes to every route file.

use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

use crate::AppState;

/// Build the full OpenAPI 3.0 spec as a JSON value.
pub fn openapi_spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "MinKy API",
            "version": "0.1.0",
            "description": "Team Knowledge Intelligence Platform - REST API.\n\nMinKy helps small teams capture tacit knowledge and make it searchable via natural language queries.",
            "contact": {
                "name": "Mario Cho",
                "email": "hephaex@gmail.com"
            },
            "license": {
                "name": "MIT"
            }
        },
        "servers": [
            { "url": "/api", "description": "Local development server" }
        ],
        "tags": [
            { "name": "health",     "description": "System health checks" },
            { "name": "auth",       "description": "Authentication and JWT management" },
            { "name": "documents",  "description": "Document CRUD operations" },
            { "name": "embeddings", "description": "Vector embedding management" },
            { "name": "search",     "description": "Semantic search and RAG queries" },
            { "name": "knowledge",  "description": "Knowledge graph and team expertise" },
            { "name": "slack",      "description": "Slack/Teams integration and knowledge extraction" },
            { "name": "understanding", "description": "AI document analysis (Claude)" }
        ],
        "paths": {
            "/health": {
                "get": {
                    "tags": ["health"],
                    "summary": "Health check",
                    "description": "Returns server and database health status.",
                    "operationId": "getHealth",
                    "responses": {
                        "200": {
                            "description": "Server is healthy",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/HealthResponse" },
                                    "example": {
                                        "status": "ok",
                                        "version": "0.1.0",
                                        "database": "healthy"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/auth/login": {
                "post": {
                    "tags": ["auth"],
                    "summary": "Login",
                    "operationId": "login",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/LoginRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Login successful",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/TokenResponse" }
                                }
                            }
                        },
                        "401": { "description": "Invalid credentials" }
                    }
                }
            },
            "/auth/register": {
                "post": {
                    "tags": ["auth"],
                    "summary": "Register new user",
                    "operationId": "register",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/RegisterRequest" }
                            }
                        }
                    },
                    "responses": {
                        "201": { "description": "User created" },
                        "409": { "description": "Email already exists" }
                    }
                }
            },
            "/documents": {
                "get": {
                    "tags": ["documents"],
                    "summary": "List documents",
                    "operationId": "listDocuments",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "page",        "in": "query", "schema": { "type": "integer", "default": 1 } },
                        { "name": "limit",       "in": "query", "schema": { "type": "integer", "default": 20 } },
                        { "name": "search",      "in": "query", "schema": { "type": "string" } },
                        { "name": "category_id", "in": "query", "schema": { "type": "integer" } }
                    ],
                    "responses": {
                        "200": { "description": "Paginated document list" }
                    }
                },
                "post": {
                    "tags": ["documents"],
                    "summary": "Create document",
                    "operationId": "createDocument",
                    "security": [{ "bearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CreateDocumentRequest" }
                            }
                        }
                    },
                    "responses": {
                        "201": { "description": "Document created" },
                        "400": { "description": "Validation error" }
                    }
                }
            },
            "/documents/{id}": {
                "get": {
                    "tags": ["documents"],
                    "summary": "Get document by ID",
                    "operationId": "getDocument",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }
                    ],
                    "responses": {
                        "200": { "description": "Document details" },
                        "404": { "description": "Not found" }
                    }
                },
                "put": {
                    "tags": ["documents"],
                    "summary": "Update document",
                    "operationId": "updateDocument",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/UpdateDocumentRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Updated document" },
                        "404": { "description": "Not found" }
                    }
                },
                "delete": {
                    "tags": ["documents"],
                    "summary": "Delete document",
                    "operationId": "deleteDocument",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }
                    ],
                    "responses": {
                        "204": { "description": "Deleted" },
                        "404": { "description": "Not found" }
                    }
                }
            },
            "/documents/{id}/understand": {
                "post": {
                    "tags": ["understanding"],
                    "summary": "Analyse document with AI",
                    "description": "Uses Claude to extract topics, summary, insights and technologies from the document.",
                    "operationId": "understandDocument",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }
                    ],
                    "responses": {
                        "200": { "description": "AI analysis result" },
                        "404": { "description": "Document not found" },
                        "502": { "description": "Anthropic API error" }
                    }
                }
            },
            "/embeddings/documents/{id}": {
                "post": {
                    "tags": ["embeddings"],
                    "summary": "Generate document embedding",
                    "description": "Creates a vector embedding for the specified document using OpenAI text-embedding-3-small.",
                    "operationId": "embedDocument",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }
                    ],
                    "responses": {
                        "200": { "description": "Embedding created" },
                        "502": { "description": "OpenAI API error" }
                    }
                }
            },
            "/embeddings/stats": {
                "get": {
                    "tags": ["embeddings"],
                    "summary": "Embedding statistics",
                    "operationId": "getEmbeddingStats",
                    "security": [{ "bearerAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Embedding stats",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/EmbeddingStats" }
                                }
                            }
                        }
                    }
                }
            },
            "/embeddings/similar/{id}": {
                "get": {
                    "tags": ["embeddings"],
                    "summary": "Find similar documents",
                    "operationId": "getSimilarDocuments",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } },
                        { "name": "limit", "in": "query", "schema": { "type": "integer", "default": 5 } }
                    ],
                    "responses": {
                        "200": { "description": "Similar document list with similarity scores" }
                    }
                }
            },
            "/search/ask": {
                "post": {
                    "tags": ["search"],
                    "summary": "RAG natural-language Q&A",
                    "description": "Ask a question in natural language. The system retrieves relevant documents via vector search and generates an answer using Claude.",
                    "operationId": "ragAsk",
                    "security": [{ "bearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/RagAskRequest" },
                                "example": {
                                    "question": "How do we handle pgvector performance issues?",
                                    "top_k": 5,
                                    "threshold": 0.7
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "AI-generated answer with source references" },
                        "502": { "description": "Anthropic or OpenAI API error" }
                    }
                }
            },
            "/search/semantic": {
                "post": {
                    "tags": ["search"],
                    "summary": "Semantic vector search",
                    "description": "Search documents by semantic similarity to a query phrase.",
                    "operationId": "semanticSearch",
                    "security": [{ "bearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SemanticSearchRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Ranked list of matching documents" }
                    }
                }
            },
            "/search/history": {
                "get": {
                    "tags": ["search"],
                    "summary": "Search history",
                    "operationId": "getSearchHistory",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "limit", "in": "query", "schema": { "type": "integer", "default": 20 } }
                    ],
                    "responses": {
                        "200": { "description": "Recent search queries" }
                    }
                }
            },
            "/knowledge/graph": {
                "get": {
                    "tags": ["knowledge"],
                    "summary": "Get knowledge graph",
                    "description": "Returns the full knowledge graph (document nodes + derived topic/technology nodes + similarity edges).",
                    "operationId": "getKnowledgeGraph",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "threshold",              "in": "query", "schema": { "type": "number", "format": "float", "default": 0.5 } },
                        { "name": "max_edges",              "in": "query", "schema": { "type": "integer", "default": 5 } },
                        { "name": "include_topics",         "in": "query", "schema": { "type": "boolean", "default": true } },
                        { "name": "include_technologies",   "in": "query", "schema": { "type": "boolean", "default": true } },
                        { "name": "include_insights",       "in": "query", "schema": { "type": "boolean", "default": false } },
                        { "name": "max_documents",          "in": "query", "schema": { "type": "integer", "default": 100 } }
                    ],
                    "responses": {
                        "200": {
                            "description": "Knowledge graph nodes and edges",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/KnowledgeGraphResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/knowledge/team": {
                "get": {
                    "tags": ["knowledge"],
                    "summary": "Team expertise map",
                    "description": "Returns per-member expertise areas, shared knowledge, and unique expert identification.",
                    "operationId": "getTeamExpertise",
                    "security": [{ "bearerAuth": [] }],
                    "responses": {
                        "200": { "description": "Team expertise map" }
                    }
                }
            },
            "/slack/extract": {
                "post": {
                    "tags": ["slack"],
                    "summary": "Extract knowledge from conversation",
                    "description": "Analyses a Slack/Teams conversation thread and extracts structured knowledge using Claude.",
                    "operationId": "slackExtract",
                    "security": [{ "bearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SlackExtractRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Extraction result" },
                        "502": { "description": "Anthropic API error" }
                    }
                }
            },
            "/slack/confirm": {
                "post": {
                    "tags": ["slack"],
                    "summary": "Confirm or reject extracted knowledge",
                    "operationId": "slackConfirm",
                    "security": [{ "bearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ConfirmKnowledgeRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Confirmation recorded" }
                    }
                }
            },
            "/slack/summary": {
                "get": {
                    "tags": ["slack"],
                    "summary": "Extraction activity summary",
                    "operationId": "slackSummary",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "since", "in": "query", "schema": { "type": "string", "format": "date-time" } }
                    ],
                    "responses": {
                        "200": { "description": "Extraction statistics" }
                    }
                }
            },
            "/slack/oauth/callback": {
                "get": {
                    "tags": ["slack"],
                    "summary": "Slack OAuth 2.0 callback",
                    "description": "Handles the redirect from Slack after workspace authorization. Exchanges the code for a bot token.",
                    "operationId": "slackOAuthCallback",
                    "parameters": [
                        { "name": "code",  "in": "query", "schema": { "type": "string" } },
                        { "name": "state", "in": "query", "schema": { "type": "string" } },
                        { "name": "error", "in": "query", "schema": { "type": "string" } }
                    ],
                    "responses": {
                        "200": { "description": "Workspace connected" },
                        "400": { "description": "Missing code parameter" },
                        "502": { "description": "Slack OAuth API error" }
                    }
                }
            },
            "/slack/webhook": {
                "post": {
                    "tags": ["slack"],
                    "summary": "Slack Events API webhook",
                    "description": "Receives Slack event payloads. Handles url_verification challenges and event_callback messages (queues knowledge extraction for message/app_mention events).",
                    "operationId": "slackWebhook",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SlackWebhookPayload" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Acknowledged" }
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT"
                }
            },
            "schemas": {
                "HealthResponse": {
                    "type": "object",
                    "properties": {
                        "status":   { "type": "string", "example": "ok" },
                        "version":  { "type": "string", "example": "0.1.0" },
                        "database": { "type": "string", "example": "healthy" }
                    }
                },
                "LoginRequest": {
                    "type": "object",
                    "required": ["email", "password"],
                    "properties": {
                        "email":    { "type": "string", "format": "email" },
                        "password": { "type": "string", "format": "password" }
                    }
                },
                "RegisterRequest": {
                    "type": "object",
                    "required": ["email", "password", "username"],
                    "properties": {
                        "email":    { "type": "string", "format": "email" },
                        "password": { "type": "string" },
                        "username": { "type": "string" }
                    }
                },
                "TokenResponse": {
                    "type": "object",
                    "properties": {
                        "access_token":  { "type": "string" },
                        "refresh_token": { "type": "string" },
                        "token_type":    { "type": "string", "example": "bearer" },
                        "expires_in":    { "type": "integer" }
                    }
                },
                "CreateDocumentRequest": {
                    "type": "object",
                    "required": ["title", "content"],
                    "properties": {
                        "title":       { "type": "string" },
                        "content":     { "type": "string" },
                        "is_public":   { "type": "boolean", "default": false },
                        "category_id": { "type": "integer" },
                        "tag_ids":     { "type": "array", "items": { "type": "integer" } }
                    }
                },
                "UpdateDocumentRequest": {
                    "type": "object",
                    "properties": {
                        "title":       { "type": "string" },
                        "content":     { "type": "string" },
                        "is_public":   { "type": "boolean" },
                        "category_id": { "type": "integer" }
                    }
                },
                "EmbeddingStats": {
                    "type": "object",
                    "properties": {
                        "total_documents": { "type": "integer" },
                        "total_chunks":    { "type": "integer" },
                        "embedding_model": { "type": "string" },
                        "dimensions":      { "type": "integer" }
                    }
                },
                "RagAskRequest": {
                    "type": "object",
                    "required": ["question"],
                    "properties": {
                        "question":  { "type": "string" },
                        "top_k":     { "type": "integer", "default": 5 },
                        "threshold": { "type": "number", "format": "float", "default": 0.7 }
                    }
                },
                "SemanticSearchRequest": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query":     { "type": "string" },
                        "limit":     { "type": "integer", "default": 10 },
                        "threshold": { "type": "number", "format": "float", "default": 0.6 }
                    }
                },
                "KnowledgeGraphResponse": {
                    "type": "object",
                    "properties": {
                        "success": { "type": "boolean" },
                        "data": {
                            "type": "object",
                            "properties": {
                                "nodes": {
                                    "type": "array",
                                    "items": { "$ref": "#/components/schemas/GraphNode" }
                                },
                                "edges": {
                                    "type": "array",
                                    "items": { "$ref": "#/components/schemas/GraphEdge" }
                                }
                            }
                        }
                    }
                },
                "GraphNode": {
                    "type": "object",
                    "properties": {
                        "id":             { "type": "string" },
                        "label":          { "type": "string" },
                        "node_type":      { "type": "string", "enum": ["document", "topic", "technology", "person", "insight"] },
                        "document_count": { "type": "integer" },
                        "summary":        { "type": "string", "nullable": true }
                    }
                },
                "GraphEdge": {
                    "type": "object",
                    "properties": {
                        "source":    { "type": "string" },
                        "target":    { "type": "string" },
                        "weight":    { "type": "number", "format": "float" },
                        "edge_type": { "type": "string", "enum": ["similar", "has_topic", "uses_technology", "has_insight", "authored_by"] }
                    }
                },
                "SlackExtractRequest": {
                    "type": "object",
                    "required": ["conversation_id", "messages"],
                    "properties": {
                        "conversation_id": { "type": "string", "example": "C01ABC123/1700000000.000" },
                        "messages": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/PlatformMessage" }
                        },
                        "filter": { "$ref": "#/components/schemas/MessageFilter" }
                    }
                },
                "PlatformMessage": {
                    "type": "object",
                    "required": ["id", "platform", "workspace_id", "channel_id", "user_id", "text", "posted_at", "captured_at"],
                    "properties": {
                        "id":           { "type": "string" },
                        "platform":     { "type": "string", "enum": ["slack", "teams", "discord"] },
                        "workspace_id": { "type": "string" },
                        "channel_id":   { "type": "string" },
                        "channel_name": { "type": "string", "nullable": true },
                        "user_id":      { "type": "string" },
                        "username":     { "type": "string", "nullable": true },
                        "text":         { "type": "string" },
                        "thread_ts":    { "type": "string", "nullable": true },
                        "reply_count":  { "type": "integer" },
                        "posted_at":    { "type": "string", "format": "date-time" },
                        "captured_at":  { "type": "string", "format": "date-time" }
                    }
                },
                "MessageFilter": {
                    "type": "object",
                    "properties": {
                        "platform":   { "type": "string" },
                        "channel_id": { "type": "string" },
                        "user_id":    { "type": "string" },
                        "since":      { "type": "string", "format": "date-time" },
                        "limit":      { "type": "integer", "default": 50 }
                    }
                },
                "ConfirmKnowledgeRequest": {
                    "type": "object",
                    "required": ["extraction_id", "confirmed"],
                    "properties": {
                        "extraction_id":     { "type": "string" },
                        "confirmed":         { "type": "boolean" },
                        "override_title":    { "type": "string", "nullable": true },
                        "override_summary":  { "type": "string", "nullable": true }
                    }
                },
                "SlackWebhookPayload": {
                    "type": "object",
                    "required": ["type"],
                    "properties": {
                        "type":      { "type": "string", "enum": ["url_verification", "event_callback"] },
                        "challenge": { "type": "string", "nullable": true },
                        "team_id":   { "type": "string", "nullable": true },
                        "event":     { "type": "object", "nullable": true }
                    }
                }
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// GET /api/docs/openapi.json
///
/// Returns the OpenAPI 3.0 specification as JSON.
async fn serve_openapi_json() -> Json<Value> {
    Json(openapi_spec())
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppState> {
    Router::new().route("/docs/openapi.json", get(serve_openapi_json))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_has_openapi_version() {
        let spec = openapi_spec();
        assert_eq!(spec["openapi"], "3.0.3");
    }

    #[test]
    fn test_openapi_spec_has_title() {
        let spec = openapi_spec();
        assert_eq!(spec["info"]["title"], "MinKy API");
    }

    #[test]
    fn test_openapi_spec_has_version() {
        let spec = openapi_spec();
        assert_eq!(spec["info"]["version"], "0.1.0");
    }

    #[test]
    fn test_openapi_spec_has_health_endpoint() {
        let spec = openapi_spec();
        assert!(spec["paths"]["/health"]["get"].is_object());
    }

    #[test]
    fn test_openapi_spec_has_documents_endpoints() {
        let spec = openapi_spec();
        assert!(spec["paths"]["/documents"]["get"].is_object());
        assert!(spec["paths"]["/documents"]["post"].is_object());
        assert!(spec["paths"]["/documents/{id}"]["get"].is_object());
        assert!(spec["paths"]["/documents/{id}"]["put"].is_object());
        assert!(spec["paths"]["/documents/{id}"]["delete"].is_object());
    }

    #[test]
    fn test_openapi_spec_has_rag_endpoints() {
        let spec = openapi_spec();
        assert!(spec["paths"]["/search/ask"]["post"].is_object());
        assert!(spec["paths"]["/search/semantic"]["post"].is_object());
        assert!(spec["paths"]["/search/history"]["get"].is_object());
    }

    #[test]
    fn test_openapi_spec_has_knowledge_endpoints() {
        let spec = openapi_spec();
        assert!(spec["paths"]["/knowledge/graph"]["get"].is_object());
        assert!(spec["paths"]["/knowledge/team"]["get"].is_object());
    }

    #[test]
    fn test_openapi_spec_has_slack_endpoints() {
        let spec = openapi_spec();
        assert!(spec["paths"]["/slack/extract"]["post"].is_object());
        assert!(spec["paths"]["/slack/confirm"]["post"].is_object());
        assert!(spec["paths"]["/slack/summary"]["get"].is_object());
        assert!(spec["paths"]["/slack/oauth/callback"]["get"].is_object());
        assert!(spec["paths"]["/slack/webhook"]["post"].is_object());
    }

    #[test]
    fn test_openapi_spec_has_bearer_security_scheme() {
        let spec = openapi_spec();
        assert_eq!(
            spec["components"]["securitySchemes"]["bearerAuth"]["scheme"],
            "bearer"
        );
    }

    #[test]
    fn test_openapi_spec_has_component_schemas() {
        let spec = openapi_spec();
        let schemas = &spec["components"]["schemas"];
        assert!(schemas["HealthResponse"].is_object());
        assert!(schemas["CreateDocumentRequest"].is_object());
        assert!(schemas["RagAskRequest"].is_object());
        assert!(schemas["GraphNode"].is_object());
        assert!(schemas["GraphEdge"].is_object());
        assert!(schemas["SlackExtractRequest"].is_object());
        assert!(schemas["PlatformMessage"].is_object());
        assert!(schemas["SlackWebhookPayload"].is_object());
    }

    #[test]
    fn test_openapi_spec_has_tags() {
        let spec = openapi_spec();
        let tags = spec["tags"].as_array().unwrap();
        let tag_names: Vec<&str> = tags
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(tag_names.contains(&"health"));
        assert!(tag_names.contains(&"documents"));
        assert!(tag_names.contains(&"slack"));
        assert!(tag_names.contains(&"knowledge"));
    }

    #[test]
    fn test_openapi_spec_server_url() {
        let spec = openapi_spec();
        let servers = spec["servers"].as_array().unwrap();
        assert_eq!(servers[0]["url"], "/api");
    }

    #[test]
    fn test_knowledge_graph_schema_has_node_types_enum() {
        let spec = openapi_spec();
        let node_types = &spec["components"]["schemas"]["GraphNode"]["properties"]["node_type"]["enum"];
        let types: Vec<&str> = node_types.as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
        assert!(types.contains(&"document"));
        assert!(types.contains(&"topic"));
        assert!(types.contains(&"technology"));
    }

    #[test]
    fn test_openapi_spec_auth_endpoints_present() {
        let spec = openapi_spec();
        assert!(spec["paths"]["/auth/login"]["post"].is_object());
        assert!(spec["paths"]["/auth/register"]["post"].is_object());
    }

    #[test]
    fn test_openapi_spec_embeddings_endpoints_present() {
        let spec = openapi_spec();
        // POST to generate embedding for a document
        assert!(spec["paths"]["/embeddings/documents/{id}"]["post"].is_object());
        // GET similar documents by vector similarity
        assert!(spec["paths"]["/embeddings/similar/{id}"]["get"].is_object());
        // GET embedding stats
        assert!(spec["paths"]["/embeddings/stats"]["get"].is_object());
    }

    #[test]
    fn test_openapi_spec_understanding_endpoint_present() {
        let spec = openapi_spec();
        assert!(spec["paths"]["/documents/{id}/understand"]["post"].is_object());
    }

    #[test]
    fn test_openapi_spec_graph_node_schema_structure() {
        let spec = openapi_spec();
        let node_schema = &spec["components"]["schemas"]["GraphNode"];
        assert_eq!(node_schema["type"], "object");
        assert!(node_schema["properties"]["id"].is_object());
        assert!(node_schema["properties"]["label"].is_object());
        assert!(node_schema["properties"]["node_type"].is_object());
    }

    #[test]
    fn test_openapi_spec_graph_edge_schema_has_edge_types() {
        let spec = openapi_spec();
        let edge_types = &spec["components"]["schemas"]["GraphEdge"]["properties"]["edge_type"]["enum"];
        let types: Vec<&str> = edge_types.as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
        assert!(types.contains(&"similar"));
        assert!(types.contains(&"has_topic"));
        assert!(types.contains(&"authored_by"));
    }

    #[test]
    fn test_openapi_spec_rag_request_schema_has_required_fields() {
        let spec = openapi_spec();
        let required = &spec["components"]["schemas"]["RagAskRequest"]["required"];
        let fields: Vec<&str> = required.as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
        assert!(fields.contains(&"question"));
    }

    #[test]
    fn test_openapi_spec_platform_message_schema_has_required() {
        let spec = openapi_spec();
        let required = &spec["components"]["schemas"]["PlatformMessage"]["required"];
        let fields: Vec<&str> = required.as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
        assert!(fields.contains(&"id"));
        assert!(fields.contains(&"platform"));
        assert!(fields.contains(&"text"));
    }

    #[test]
    fn test_openapi_spec_slack_webhook_schema_type_enum() {
        let spec = openapi_spec();
        let type_enum = &spec["components"]["schemas"]["SlackWebhookPayload"]["properties"]["type"]["enum"];
        let types: Vec<&str> = type_enum.as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
        assert!(types.contains(&"url_verification"));
        assert!(types.contains(&"event_callback"));
    }

    #[test]
    fn test_openapi_spec_has_contact_info() {
        let spec = openapi_spec();
        assert!(spec["info"]["contact"]["email"].as_str().unwrap().contains("@"));
    }

    #[test]
    fn test_openapi_spec_has_license() {
        let spec = openapi_spec();
        assert_eq!(spec["info"]["license"]["name"], "MIT");
    }
}
