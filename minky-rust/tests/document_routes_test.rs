use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
    Router,
};
use tower::ServiceExt;

fn document_routes_router() -> Router {
    Router::new()
        .route("/", get(|| async { "list" }).post(|| async { "create" }))
        .route("/{id}", get(|| async { "get" }).put(|| async { "update" }).delete(|| async { "delete" }))
        .route("/{id}/status", get(|| async { "status" }))
        .route("/{id}/reprocess", post(|| async { "reprocess" }))
}

#[tokio::test]
async fn status_route_returns_200() {
    let req = Request::builder()
        .uri("/550e8400-e29b-41d4-a716-446655440000/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = document_routes_router().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn reprocess_route_returns_200() {
    let req = Request::builder()
        .uri("/550e8400-e29b-41d4-a716-446655440000/reprocess")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = document_routes_router().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn status_route_rejects_post() {
    let req = Request::builder()
        .uri("/550e8400-e29b-41d4-a716-446655440000/status")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = document_routes_router().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn reprocess_route_rejects_get() {
    let req = Request::builder()
        .uri("/550e8400-e29b-41d4-a716-446655440000/reprocess")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = document_routes_router().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn unknown_sub_route_returns_404() {
    let req = Request::builder()
        .uri("/550e8400-e29b-41d4-a716-446655440000/nonexistent")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = document_routes_router().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
