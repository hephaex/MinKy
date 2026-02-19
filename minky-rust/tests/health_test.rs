//! Integration tests for the health endpoint.
//!
//! These tests exercise the Axum router without a database by using the
//! embedded health route which only returns a static JSON response.

mod common;

use axum::{routing::get, Json, Router};
use serde_json::json;

// ---------------------------------------------------------------------------
// Minimal health router for testing (mirrors the real one)
// ---------------------------------------------------------------------------

async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "version": "0.1.0",
        "database": "healthy"
    }))
}

fn test_router() -> Router {
    Router::new().route("/api/health", get(health_handler))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_health_returns_200() {
    let app = common::TestApp::new(test_router());
    let (status, body) = app.get("/api/health").await;

    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_health_returns_version() {
    let app = common::TestApp::new(test_router());
    let (_, body) = app.get("/api/health").await;

    assert!(body["version"].is_string());
    assert!(!body["version"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_health_returns_database_status() {
    let app = common::TestApp::new(test_router());
    let (_, body) = app.get("/api/health").await;

    assert!(body["database"].is_string());
}

#[tokio::test]
async fn test_unknown_route_returns_404() {
    let app = common::TestApp::new(test_router());
    let (status, _) = app.get("/api/nonexistent").await;

    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}
