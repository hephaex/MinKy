pub mod config;
pub mod error;
pub mod middleware;
pub mod models;
pub mod openapi;
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

use crate::config::Config;
use crate::middleware::rate_limit_middleware;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Config,
}

/// Create the application with all routes and middleware
pub async fn create_app(config: Config) -> Result<Router> {
    // Create database connection pool
    let db = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&db).await?;

    let state = AppState {
        db,
        config: config.clone(),
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
