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
    /// Redis client for rate limiting and JWT revocation. None when REDIS_URL is unset.
    pub redis_client: Option<redis::Client>,
    /// Shared HTTP client for external API calls (Solar, etc.). Created once at startup.
    pub http_client: reqwest::Client,
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
    let upstage_key = std::env::var("UPSTAGE_API_KEY").ok();
    let default_model = if upstage_key.is_some() {
        EmbeddingModel::UpstaageSolarPassage
    } else {
        EmbeddingModel::OpenaiTextEmbedding3Small
    };
    let embedding_config = EmbeddingConfig {
        openai_api_key: config.openai_api_key.as_ref().map(|k| k.expose_secret().to_owned()),
        voyage_api_key: None,
        upstage_api_key: upstage_key,
        default_model,
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

    // Start the embedding queue background worker unless explicitly disabled.
    // EMBEDDING_QUEUE_ENABLED=false disables it; EMBEDDING_QUEUE_INTERVAL_SECS sets the
    // idle sleep between polls when the queue is empty (default 10 s, minimum 1 s).
    let queue_enabled = std::env::var("EMBEDDING_QUEUE_ENABLED")
        .map(|v| v.to_lowercase() != "false")
        .unwrap_or(true);
    if queue_enabled {
        let idle_secs: u64 = std::env::var("EMBEDDING_QUEUE_INTERVAL_SECS")
            .ok()
            .and_then(|s| {
                s.parse::<u64>().map_err(|e| {
                    tracing::warn!(value = %s, error = %e, "EMBEDDING_QUEUE_INTERVAL_SECS invalid, using default 10 s");
                }).ok()
            })
            .map(|v| v.max(1))  // prevent busy-spin at 0
            .unwrap_or(10);
        let worker = Arc::clone(&embedding_service);
        // Supervisor: restart the worker on panic so queue processing survives crashes.
        tokio::spawn(async move {
            loop {
                let w = Arc::clone(&worker);
                let handle = tokio::spawn(async move { w.run_queue_worker(idle_secs).await });
                match handle.await {
                    Ok(_) => tracing::warn!("Embedding queue worker exited unexpectedly, restarting in 5 s"),
                    Err(e) => tracing::error!(error = %e, "Embedding queue worker panicked, restarting in 5 s"),
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });
        tracing::info!(idle_interval_secs = idle_secs, "Embedding queue worker (supervised) started");
    }

    let redis_client = std::env::var("REDIS_URL")
        .ok()
        .and_then(|url| redis::Client::open(url).ok());

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("failed to build shared HTTP client");

    let state = AppState {
        db,
        config: config.clone(),
        ws_manager,
        embedding_service,
        vault_watcher,
        redis_client,
        http_client,
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
