mod admin;
mod agents;
mod ai;
mod analytics;
mod attachments;
mod auth;
mod categories;
mod comments;
pub mod documents;
pub mod embeddings;
mod export;
mod git;
mod harness;
pub mod health;
pub mod knowledge;
mod korean;
mod ml;
mod notifications;
mod ocr;
pub mod rag;
mod search;
mod security;
mod skills;
pub mod slack;
mod sync;
mod tags;
mod templates;
mod timeline;
mod understanding;
mod versions;
mod workflows;

use axum::Router;

use crate::AppState;

/// Combine all API routes
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .nest("/auth", auth::routes())
        .nest("/documents", documents::routes().merge(understanding::routes()))
        .nest("/tags", tags::routes())
        .nest("/categories", categories::routes())
        .nest("/comments", comments::routes())
        .nest("/workflows", workflows::routes())
        .nest("/attachments", attachments::routes())
        .nest("/versions", versions::routes())
        .nest("/notifications", notifications::routes())
        .nest("/ai", ai::routes())
        .nest("/search", search::routes().merge(rag::router()))
        .nest("/analytics", analytics::routes())
        .nest("/admin", admin::routes())
        .nest("/export", export::routes())
        .nest("/templates", templates::routes())
        .nest("/git", git::routes())
        .nest("/security", security::routes())
        .nest("/agents", agents::router())
        .nest("/ocr", ocr::router())
        .nest("/ml", ml::router())
        .nest("/timeline", timeline::router())
        .nest("/sync", sync::router())
        .nest("/korean", korean::router())
        .nest("/skills", skills::router())
        .nest("/harness", harness::router())
        .nest("/knowledge", knowledge::router())
        .nest("/embeddings", embeddings::router())
        .nest("/slack", slack::router())
        .merge(crate::openapi::router())
}
