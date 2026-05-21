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
use crate::services::{EmbeddingConfig, EmbeddingService, WebSocketManager};
use crate::models::EmbeddingModel;

/// Application state shared across all handlers.
///
/// `vault_watcher` wraps the handle in `Arc<Mutex<Option<…>>>` so that:
/// - `AppState` can derive `Clone` cheaply (Arc clone).
/// - The reload endpoint can swap the handle under the mutex without needing
///   a mutable reference to the whole state.
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Config,
    pub ws_manager: Arc<WebSocketManager>,
    pub embedding_service: Arc<EmbeddingService>,
    pub vault_watcher: Arc<tokio::sync::Mutex<Option<crate::services::vault_watcher_service::VaultWatcherHandle>>>,
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

    // Initialize embedding service once at startup. When local_embedding_enabled
    // is true this downloads ~270 MB (nomic-embed-text-v1.5) on first run.
    use secrecy::ExposeSecret;
    let embedding_config = EmbeddingConfig {
        openai_api_key: config.openai_api_key.as_ref().map(|k| k.expose_secret().to_owned()),
        voyage_api_key: None,
        default_model: EmbeddingModel::OpenaiTextEmbedding3Small,
        chunk_size: 512,
        chunk_overlap: 50,
        local_embedding_enabled: config.local_embedding_enabled,
    };
    let embedding_service = Arc::new(EmbeddingService::new_with_local(db.clone(), embedding_config).await?);

    // Optionally start the vault file-system watcher based on configuration.
    let vault_watcher = if config.vault_watch.enabled {
        if let Some(user_id) = config.vault_watch.user_id {
            let watcher_svc = crate::services::vault_watcher_service::VaultWatcherService::new(
                db.clone(),
                Arc::clone(&embedding_service),
            );
            match watcher_svc
                .start(config.vault_watch.roots.clone(), user_id, config.vault_watch.initial_scan)
                .await
            {
                Ok(handle) => {
                    tracing::info!(
                        root_count = config.vault_watch.roots.len(),
                        "Vault watcher started"
                    );
                    Arc::new(tokio::sync::Mutex::new(Some(handle)))
                }
                Err(e) => {
                    tracing::error!("Failed to start vault watcher: {}", e);
                    Arc::new(tokio::sync::Mutex::new(None))
                }
            }
        } else {
            tracing::warn!(
                "vault_watch.enabled=true but user_id not set — watcher not started"
            );
            Arc::new(tokio::sync::Mutex::new(None))
        }
    } else {
        Arc::new(tokio::sync::Mutex::new(None))
    };

    let state = AppState {
        db,
        config: config.clone(),
        ws_manager,
        embedding_service,
        vault_watcher,
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
