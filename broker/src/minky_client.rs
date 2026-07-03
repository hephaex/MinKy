//! Semantic-search HTTP client for the long-term memory backend (minky).
//!
//! Vendor-neutral contract (measured against `minky-rust`): the broker only
//! ever asks for evidence — raw snippets plus their source — via
//! `POST {base_url}/api/search/semantic`. It never calls `/search/ask` and
//! never lets the backend synthesize an answer (approved plan WP3).
//!
//! Request body: `{"query": String, "limit": i32, "threshold": f32}` with the
//! long-lived M2M key in the `X-API-Key` header. Response envelope:
//! `{"success": bool, "data": {"results": [...], "total": usize, "query": ..}}`
//! where `results` is ordered by descending similarity.
//!
//! Failure taxonomy (approved plan WP3):
//! - connection refused / DNS / timeout → [`MinkyError::Unreachable`] (the
//!   offline-degradation trigger; verdict mapping lives in [`crate::cascade`]).
//! - any non-2xx status → [`MinkyError::Http`] (a real error, propagated).

use crate::config::{MinkyConfig, VerdictConfig};
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum MinkyError {
    #[error("failed to read minky api key file {0}: {1}")]
    ApiKeyRead(PathBuf, #[source] std::io::Error),
    #[error("failed to build minky http client: {0}")]
    Build(String),
    #[error("minky endpoint unreachable: {0}")]
    Unreachable(String),
    #[error("minky returned HTTP {0}")]
    Http(u16),
    #[error("failed to decode minky response: {0}")]
    Decode(String),
}

/// One semantic-search hit (vendor-neutral projection of the minky contract).
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticHit {
    pub document_id: String,
    pub chunk_id: Option<String>,
    pub chunk_text: Option<String>,
    pub similarity: f32,
    pub document_title: Option<String>,
}

/// `{"success": .., "data": { .. }}`
#[derive(Debug, Deserialize)]
struct SemanticEnvelope {
    data: SemanticData,
}

/// `{"results": [..], "total": .., "query": ..}`
#[derive(Debug, Deserialize)]
struct SemanticData {
    #[serde(default)]
    results: Vec<SemanticResultRaw>,
}

#[derive(Debug, Deserialize)]
struct SemanticResultRaw {
    document_id: String,
    #[serde(default)]
    chunk_id: Option<String>,
    #[serde(default)]
    chunk_text: Option<String>,
    similarity: f32,
    #[serde(default)]
    document_title: Option<String>,
}

/// Stateless (per-call) client: no daemon, no background connection. The
/// cascade constructs it once and calls [`MinkyClient::search_semantic`] on
/// demand (approved plan: no resident processes).
pub struct MinkyClient {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

impl MinkyClient {
    /// Read the M2M key from `config.api_key_file` (trimmed of surrounding
    /// whitespace/newline) and build a client with the configured timeout.
    pub fn new(config: &MinkyConfig) -> Result<Self, MinkyError> {
        let raw = std::fs::read_to_string(&config.api_key_file)
            .map_err(|e| MinkyError::ApiKeyRead(config.api_key_file.clone(), e))?;
        let api_key = raw.trim().to_string();

        let timeout = Duration::from_secs(config.timeout_secs);
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(timeout)
            .build()
            .map_err(|e| MinkyError::Build(e.to_string()))?;

        Ok(Self {
            // Normalize so `{base}/api/...` never produces a double slash.
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key,
            http,
        })
    }

    /// POST `/api/search/semantic`. `limit` and `threshold` come straight from
    /// the externalized verdict config (`top_k`, `min_score`) — no thresholds
    /// are baked in here.
    pub async fn search_semantic(
        &self,
        query: &str,
        verdict: &VerdictConfig,
    ) -> Result<Vec<SemanticHit>, MinkyError> {
        let url = format!("{}/api/search/semantic", self.base_url);
        let body = serde_json::json!({
            "query": query,
            "limit": verdict.top_k,
            "threshold": verdict.min_score,
        });

        let resp = self
            .http
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&body)
            .send()
            .await
            // send() failures are connect/DNS/timeout/body-stream problems —
            // treat them all as "the backend is not reachable right now".
            .map_err(|e| MinkyError::Unreachable(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(MinkyError::Http(status.as_u16()));
        }

        let envelope: SemanticEnvelope = resp
            .json()
            .await
            .map_err(|e| MinkyError::Decode(e.to_string()))?;

        Ok(envelope
            .data
            .results
            .into_iter()
            .map(|r| SemanticHit {
                document_id: r.document_id,
                chunk_id: r.chunk_id,
                chunk_text: r.chunk_text,
                similarity: r.similarity,
                document_title: r.document_title,
            })
            .collect())
    }
}
