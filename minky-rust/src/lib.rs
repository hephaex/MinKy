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
use crate::middleware::{fix_multipart_boundary, rate_limit_middleware};
use crate::services::{EmbeddingConfig, EmbeddingService, WebSocketManager};
use crate::models::EmbeddingModel;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Config,
    pub ws_manager: Arc<WebSocketManager>,
    pub embedding_service: Arc<EmbeddingService>,
    pub vault_watcher: Arc<tokio::sync::Mutex<Option<crate::services::vault_watcher_service::VaultWatcherHandle>>>,
    pub redis_client: Option<redis::Client>,
    pub http_client: reqwest::Client,
    /// Shared mecab-ko morphological tokenizer, loaded once at startup from
    /// `MECAB_KO_DICDIR` if set. `None` when unset or loading failed;
    /// `KoreanService` falls back to whitespace tokenization in that case.
    pub korean_tokenizer: Option<crate::services::SharedKoreanTokenizer>,
}

pub async fn create_app(config: Config) -> Result<Router> {
    let mut pool_options = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .min_connections(config.database_min_connections)
        .acquire_timeout(std::time::Duration::from_secs(config.database_acquire_timeout_secs));

    if config.database_max_lifetime_secs > 0 {
        pool_options = pool_options
            .max_lifetime(std::time::Duration::from_secs(config.database_max_lifetime_secs));
    }

    if config.database_idle_timeout_secs > 0 {
        pool_options = pool_options
            .idle_timeout(std::time::Duration::from_secs(config.database_idle_timeout_secs));
    }

    let db = pool_options
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;

    let ws_manager = Arc::new(WebSocketManager::new());

    use secrecy::ExposeSecret;
    // Default embedding model is pinned to UpstaageSolarPassage: all real
    // chunk_embeddings/document_embeddings rows are already tagged with
    // this model (verified 2026-09-21, 4320 non-archived rows), because
    // EMBEDDING_BASE_URL is overridden to Strix Halo's OpenAI-compatible
    // endpoint, which needs no API key. Selecting OpenaiTextEmbedding3Small
    // whenever UPSTAGE_API_KEY happened to be unset or empty (the previous
    // behavior) tagged every newly generated embedding with the wrong
    // model, breaking the WHERE ce.model = $2 filter in
    // EmbeddingService::semantic_search for both writes and reads.
    //
    // The placeholder key below is only used to satisfy the is_upstage()
    // fail-fast check in EmbeddingService::generate_embedding; the actual
    // HTTP call goes to Strix Halo via EMBEDDING_BASE_URL, which ignores
    // the Authorization header entirely.
    let upstage_key = std::env::var("UPSTAGE_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .or_else(|| Some("unused-strix-endpoint-ignores-this".to_string()));
    let default_model = EmbeddingModel::UpstaageSolarPassage;
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
                    tracing::info!(root_count = config.vault_watch.roots.len(), "Vault watcher started");
                    Arc::new(tokio::sync::Mutex::new(Some(handle)))
                }
                Err(e) => {
                    tracing::error!("Failed to start vault watcher: {}", e);
                    Arc::new(tokio::sync::Mutex::new(None))
                }
            }
        } else {
            tracing::warn!("vault_watch.enabled=true but user_id not set — watcher not started");
            Arc::new(tokio::sync::Mutex::new(None))
        }
    } else {
        Arc::new(tokio::sync::Mutex::new(None))
    };

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
            .map(|v| v.max(1))
            .unwrap_or(10);
        let worker = Arc::clone(&embedding_service);
        tokio::spawn(async move { worker.run_queue_worker(idle_secs).await });
        tracing::info!(idle_interval_secs = idle_secs, "Embedding queue worker started");
    }

    let redis_client = std::env::var("REDIS_URL")
        .ok()
        .and_then(|url| redis::Client::open(url).ok());

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("failed to build shared HTTP client");

    // Load the mecab-ko morphological tokenizer once at startup. This is a
    // synchronous, potentially-blocking disk load (mmap + zstd decompress of
    // the dictionary produced by mecab-ko-dict-builder), offloaded to
    // spawn_blocking so it doesn't stall the async runtime during startup.
    let korean_tokenizer = tokio::task::spawn_blocking(
        crate::services::load_shared_korean_tokenizer,
    )
    .await
    .unwrap_or_else(|e| {
        tracing::error!(error = %e, "mecab-ko tokenizer load task panicked; falling back to whitespace tokenization");
        None
    });

    let state = AppState {
        db,
        config: config.clone(),
        ws_manager,
        embedding_service,
        vault_watcher,
        redis_client,
        http_client,
        korean_tokenizer,
    };

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

    let app = Router::new()
        .nest("/api", routes::api_routes())
        .with_state(state)
        .layer(axum_middleware::from_fn(fix_multipart_boundary))
        .layer(axum_middleware::from_fn(rate_limit_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors_layer);

    Ok(app)
}
