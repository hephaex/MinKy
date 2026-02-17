use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}

async fn health_check(State(state): State<AppState>) -> Json<Value> {
    // Check database connection
    let db_status = sqlx::query("SELECT 1")
        .fetch_one(&state.db)
        .await
        .map(|_| "healthy")
        .unwrap_or("unhealthy");

    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "database": db_status
    }))
}
