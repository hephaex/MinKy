mod attachments;
mod auth;
mod categories;
mod comments;
mod documents;
mod health;
mod notifications;
mod tags;
mod versions;
mod workflows;

use axum::Router;

use crate::AppState;

/// Combine all API routes
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .nest("/auth", auth::routes())
        .nest("/documents", documents::routes())
        .nest("/tags", tags::routes())
        .nest("/categories", categories::routes())
        .nest("/comments", comments::routes())
        .nest("/workflows", workflows::routes())
        .nest("/attachments", attachments::routes())
        .nest("/versions", versions::routes())
        .nest("/notifications", notifications::routes())
}
