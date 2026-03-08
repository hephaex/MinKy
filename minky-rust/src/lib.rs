pub mod config;
pub mod error;
pub mod middleware;
pub mod models;
pub mod openapi;
pub mod pipeline;
pub mod routes;
pub mod services;
pub mod utils;

use anyhow::Result;
use axum::{middleware as axum_middleware, Router};
use axum::http::{header, Method};
use sqlx::postgres::PgPoolOptions;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::TraceLayer,
};

use std::sync::Arc;

use crate::config::Config;
use crate::middleware::rate_limit_middleware;
use crate::services::WebSocketManager;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Config,
    pub ws_manager: Arc<WebSocketManager>,
}

/// Create the application with all routes and middleware
pub async fn create_app(config: Config) -> Result<Router> {
    // Create database connection pool with production-ready settings
    let mut pool_options = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .min_connections(config.database_min_connections)
        .acquire_timeout(std::time::Duration::from_secs(config.database_acquire_timeout_secs));

    // Apply max lifetime if set (0 = unlimited)
    if config.database_max_lifetime_secs > 0 {
        pool_options = pool_options
            .max_lifetime(std::time::Duration::from_secs(config.database_max_lifetime_secs));
    }

    // Apply idle timeout if set (0 = unlimited)
    if config.database_idle_timeout_secs > 0 {
        pool_options = pool_options
            .idle_timeout(std::time::Duration::from_secs(config.database_idle_timeout_secs));
    }

    let db = pool_options
        .connect(&config.database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&db).await?;

    let ws_manager = Arc::new(WebSocketManager::new());

    let state = AppState {
        db,
        config: config.clone(),
        ws_manager,
    };

    // Parse CORS allowed origins from config
    let cors_origins: Vec<_> = config
        .cors_allowed_origins
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let cors_layer = CorsLayer::new()
        .allow_origin(cors_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
        ])
        .allow_credentials(true);

    // Build router with all routes and middleware
    let app = Router::new()
        .nest("/api", routes::api_routes())
        .with_state(state)
        .layer(axum_middleware::from_fn(rate_limit_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors_layer);

    Ok(app)
}
