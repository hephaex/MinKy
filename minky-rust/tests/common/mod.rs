//! Shared helpers for integration tests.
//!
//! This module provides:
//! - `TestApp` – a fully configured Axum test server backed by a real
//!   (or mock) database pool.
//! - JSON assertion helpers
//! - Seed data builders

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde_json::Value;
use tower::ServiceExt;

/// Minimal test application wrapper.
///
/// Wraps an Axum `Router` so tests can issue HTTP requests without
/// spinning up a real TCP listener.
#[allow(dead_code)]
pub struct TestApp {
    router: Router,
}

#[allow(dead_code)]
impl TestApp {
    /// Create a `TestApp` from any `Router`.
    pub fn new(router: Router) -> Self {
        Self { router }
    }

    /// Send a GET request and return (status, body).
    pub async fn get(&self, path: &str) -> (StatusCode, Value) {
        let req = Request::builder()
            .uri(path)
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = self.router.clone().oneshot(req).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, body)
    }

    /// Send a POST request with a JSON body and return (status, body).
    pub async fn post(&self, path: &str, payload: Value) -> (StatusCode, Value) {
        let body = serde_json::to_vec(&payload).unwrap();
        let req = Request::builder()
            .uri(path)
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let response = self.router.clone().oneshot(req).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, body)
    }
}

/// Assert that a JSON object has `"success": true`.
#[macro_export]
macro_rules! assert_success {
    ($body:expr) => {
        assert_eq!(
            $body["success"],
            serde_json::Value::Bool(true),
            "Expected success=true, got: {}",
            $body
        );
    };
}

/// Assert that a JSON object has `"success": false`.
#[macro_export]
macro_rules! assert_error {
    ($body:expr) => {
        assert_eq!(
            $body["success"],
            serde_json::Value::Bool(false),
            "Expected success=false, got: {}",
            $body
        );
    };
}
