//! Integration tests for the upload route's DefaultBodyLimit.
//!
//! These tests verify that Axum's DefaultBodyLimit middleware correctly
//! rejects oversized request bodies before the handler runs.

mod common;

use axum::{
    body::Body,
    extract::DefaultBodyLimit,
    http::{Request, StatusCode},
    routing::post,
    Router,
};
use tower::ServiceExt;

const TEST_BODY_LIMIT: usize = 1024; // 1KB for fast tests

async fn echo_handler(body: axum::body::Bytes) -> String {
    format!("received {} bytes", body.len())
}

fn test_router() -> Router {
    let upload_route = Router::new()
        .route("/upload", post(echo_handler))
        .layer(DefaultBodyLimit::max(TEST_BODY_LIMIT));

    Router::new().merge(upload_route)
}

fn multipart_body(content: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----TestBoundary1234";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"test.md\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let content_type = format!("multipart/form-data; boundary={boundary}");
    (content_type, body)
}

#[tokio::test]
async fn upload_within_limit_returns_200() {
    let small = vec![b'a'; 100];
    let (content_type, body) = multipart_body(&small);

    let req = Request::builder()
        .uri("/upload")
        .method("POST")
        .header("content-type", content_type)
        .body(Body::from(body))
        .unwrap();

    let response = test_router().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn upload_exceeding_limit_returns_413() {
    let oversized = vec![b'a'; TEST_BODY_LIMIT + 512];
    let (content_type, body) = multipart_body(&oversized);

    let req = Request::builder()
        .uri("/upload")
        .method("POST")
        .header("content-type", content_type)
        .body(Body::from(body))
        .unwrap();

    let response = test_router().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn upload_at_exact_limit_returns_200() {
    let probe = multipart_body(b"");
    let overhead = probe.1.len();
    let content_size = TEST_BODY_LIMIT.saturating_sub(overhead);
    let exact = vec![b'a'; content_size];
    let (content_type, body) = multipart_body(&exact);

    assert!(body.len() <= TEST_BODY_LIMIT, "body {} > limit {}", body.len(), TEST_BODY_LIMIT);

    let req = Request::builder()
        .uri("/upload")
        .method("POST")
        .header("content-type", content_type)
        .body(Body::from(body))
        .unwrap();

    let response = test_router().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn non_upload_route_returns_404() {
    let req = Request::builder()
        .uri("/nonexistent")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = test_router().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
