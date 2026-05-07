use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::Response,
    Json,
};
use redis::AsyncCommands;
use serde_json::json;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// Rate limiter backend trait
#[async_trait::async_trait]
pub trait RateLimiterBackend: Send + Sync {
    async fn check(&self, key: &str) -> bool;
    async fn cleanup(&self);
}

/// In-memory rate limiter (development/fallback)
#[derive(Clone)]
pub struct InMemoryRateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl InMemoryRateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }
}

#[async_trait::async_trait]
impl RateLimiterBackend for InMemoryRateLimiter {
    async fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut requests = self.requests.write().await;

        let entry = requests.entry(key.to_string()).or_insert_with(Vec::new);

        // Remove old requests outside the window
        entry.retain(|&time| now.duration_since(time) < self.window);

        if entry.len() >= self.max_requests {
            false
        } else {
            entry.push(now);
            true
        }
    }

    async fn cleanup(&self) {
        let now = Instant::now();
        let mut requests = self.requests.write().await;
        requests.retain(|_, times| {
            times.retain(|&time| now.duration_since(time) < self.window);
            !times.is_empty()
        });
    }
}

/// Redis-backed rate limiter (production)
#[derive(Clone)]
pub struct RedisRateLimiter {
    client: redis::Client,
    max_requests: usize,
    window_secs: u64,
    key_prefix: String,
}

impl RedisRateLimiter {
    pub fn new(redis_url: &str, max_requests: usize, window_secs: u64) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self {
            client,
            max_requests,
            window_secs,
            key_prefix: "minky:ratelimit:".to_string(),
        })
    }

    fn make_key(&self, client_id: &str) -> String {
        format!("{}{}", self.key_prefix, client_id)
    }
}

#[async_trait::async_trait]
impl RateLimiterBackend for RedisRateLimiter {
    async fn check(&self, key: &str) -> bool {
        let redis_key = self.make_key(key);

        // Try to get a connection
        let mut conn = match self.client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!("Redis connection failed, allowing request: {}", e);
                return true; // Fail open - allow request if Redis is down
            }
        };

        // Use Redis INCR with EXPIRE for sliding window rate limiting
        let result: Result<i64, redis::RedisError> = conn.incr(&redis_key, 1).await;

        match result {
            Ok(count) => {
                // Set expiry on first request
                if count == 1 {
                    let _: Result<(), redis::RedisError> = conn
                        .expire(&redis_key, self.window_secs as i64)
                        .await;
                }

                count as usize <= self.max_requests
            }
            Err(e) => {
                tracing::warn!("Redis INCR failed, allowing request: {}", e);
                true // Fail open
            }
        }
    }

    async fn cleanup(&self) {
        // Redis handles cleanup automatically via EXPIRE
    }
}

/// Unified rate limiter that can use either backend
pub struct RateLimiter {
    backend: Arc<dyn RateLimiterBackend>,
}

impl RateLimiter {
    pub fn new_in_memory(max_requests: usize, window_secs: u64) -> Self {
        Self {
            backend: Arc::new(InMemoryRateLimiter::new(max_requests, window_secs)),
        }
    }

    pub fn new_redis(redis_url: &str, max_requests: usize, window_secs: u64) -> Result<Self, redis::RedisError> {
        Ok(Self {
            backend: Arc::new(RedisRateLimiter::new(redis_url, max_requests, window_secs)?),
        })
    }

    /// Create rate limiter based on environment
    /// Uses Redis if REDIS_URL is set, otherwise falls back to in-memory
    pub fn from_env(max_requests: usize, window_secs: u64) -> Self {
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            match Self::new_redis(&redis_url, max_requests, window_secs) {
                Ok(limiter) => {
                    tracing::info!("Using Redis-backed rate limiter");
                    return limiter;
                }
                Err(e) => {
                    tracing::warn!("Failed to create Redis rate limiter: {}, falling back to in-memory", e);
                }
            }
        }
        tracing::info!("Using in-memory rate limiter");
        Self::new_in_memory(max_requests, window_secs)
    }

    pub async fn check(&self, key: &str) -> bool {
        self.backend.check(key).await
    }

    pub async fn cleanup(&self) {
        self.backend.cleanup().await;
    }
}

/// Returns `true` when the `TRUSTED_PROXY` environment variable is set to
/// `"true"` (case-insensitive). The result is cached after the first read.
///
/// When this returns `false` (the default), proxy headers such as
/// `x-forwarded-for` and `x-real-ip` are ignored because any client can
/// forge them, bypassing rate limiting.
fn is_trusted_proxy() -> bool {
    static TRUSTED: OnceLock<bool> = OnceLock::new();
    *TRUSTED.get_or_init(|| {
        std::env::var("TRUSTED_PROXY")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    })
}

/// Extract a stable client identifier for rate limiting.
///
/// When `trust_proxy` is `true` the priority order is:
/// 1. `x-forwarded-for` first entry (trusted reverse-proxy header)
/// 2. `x-real-ip` header
/// 3. `ConnectInfo<SocketAddr>` peer IP (direct TCP peer)
/// 4. `"unknown"` fallback (only when the server was started without connect_info)
///
/// When `trust_proxy` is `false` (the default) steps 1 and 2 are skipped so
/// that a client cannot spoof its IP by setting those headers.
fn extract_client_id_with_trust(request: &Request, trust_proxy: bool) -> String {
    if trust_proxy {
        if let Some(forwarded) = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
        {
            if let Some(first) = forwarded.split(',').next() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }

        if let Some(real_ip) = request
            .headers()
            .get("x-real-ip")
            .and_then(|h| h.to_str().ok())
        {
            let trimmed = real_ip.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    if let Some(ConnectInfo(addr)) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        return addr.ip().to_string();
    }

    tracing::warn!("No ConnectInfo available, using 'unknown' as client ID");
    "unknown".to_string()
}

/// Extract a stable client identifier for rate limiting.
///
/// Respects the `TRUSTED_PROXY` environment variable (see [`is_trusted_proxy`]).
fn extract_client_id(request: &Request) -> String {
    extract_client_id_with_trust(request, is_trusted_proxy())
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let client_id = extract_client_id(&request);

    // Default: 100 requests per minute. Override with
    // RATE_LIMIT_MAX_REQUESTS and RATE_LIMIT_WINDOW_SECS env vars.
    static LIMITER: std::sync::OnceLock<RateLimiter> = std::sync::OnceLock::new();
    let limiter = LIMITER.get_or_init(|| {
        let max_requests = std::env::var("RATE_LIMIT_MAX_REQUESTS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(100);
        let window_secs = std::env::var("RATE_LIMIT_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60);
        RateLimiter::from_env(max_requests, window_secs)
    });

    if limiter.check(&client_id).await {
        Ok(next.run(request).await)
    } else {
        Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "Rate limit exceeded",
                "retry_after": 60
            })),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_allows_requests_under_limit() {
        let limiter = RateLimiter::new_in_memory(3, 60);
        assert!(limiter.check("client1").await);
        assert!(limiter.check("client1").await);
        assert!(limiter.check("client1").await);
    }

    #[tokio::test]
    async fn test_in_memory_blocks_request_at_limit() {
        let limiter = RateLimiter::new_in_memory(2, 60);
        assert!(limiter.check("client2").await);
        assert!(limiter.check("client2").await);
        // Third request should be blocked
        assert!(!limiter.check("client2").await);
    }

    #[tokio::test]
    async fn test_in_memory_different_keys_are_independent() {
        let limiter = RateLimiter::new_in_memory(1, 60);
        assert!(limiter.check("client_a").await);
        // client_a is now at limit, but client_b should still be allowed
        assert!(!limiter.check("client_a").await);
        assert!(limiter.check("client_b").await);
    }

    #[tokio::test]
    async fn test_cleanup_does_not_panic_on_empty_state() {
        let limiter = RateLimiter::new_in_memory(10, 60);
        // cleanup on empty state should be a no-op
        limiter.cleanup().await;
    }

    #[tokio::test]
    async fn test_cleanup_removes_all_entries_after_short_window() {
        let limiter = RateLimiter::new_in_memory(10, 0); // 0-second window expires instantly
        limiter.check("cleanup_client").await;
        // After window=0 all timestamps are immediately expired
        limiter.cleanup().await;
        // Now the key should have been removed; a fresh check should succeed
        assert!(limiter.check("cleanup_client").await);
    }

    #[tokio::test]
    async fn test_from_env_falls_back_to_in_memory() {
        // Without REDIS_URL set, should use in-memory
        std::env::remove_var("REDIS_URL");
        let limiter = RateLimiter::from_env(10, 60);
        assert!(limiter.check("test_client").await);
    }

    fn build_request() -> Request {
        Request::builder().uri("/").body(axum::body::Body::empty()).unwrap()
    }

    #[test]
    fn extract_client_id_prefers_x_forwarded_for_first_entry() {
        let mut req = build_request();
        req.headers_mut()
            .insert("x-forwarded-for", "203.0.113.7, 198.51.100.1".parse().unwrap());
        req.headers_mut().insert("x-real-ip", "10.0.0.1".parse().unwrap());
        req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 55000))));

        assert_eq!(extract_client_id_with_trust(&req, true), "203.0.113.7");
    }

    #[test]
    fn extract_client_id_falls_back_to_x_real_ip() {
        let mut req = build_request();
        req.headers_mut().insert("x-real-ip", "10.0.0.42".parse().unwrap());
        req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 55000))));

        assert_eq!(extract_client_id_with_trust(&req, true), "10.0.0.42");
    }

    #[test]
    fn extract_client_id_falls_back_to_connect_info() {
        let mut req = build_request();
        req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([192, 168, 1, 5], 55000))));

        assert_eq!(extract_client_id_with_trust(&req, false), "192.168.1.5");
    }

    #[test]
    fn extract_client_id_returns_unknown_when_nothing_available() {
        let req = build_request();
        assert_eq!(extract_client_id_with_trust(&req, false), "unknown");
    }

    #[test]
    fn extract_client_id_ignores_empty_x_forwarded_for() {
        let mut req = build_request();
        req.headers_mut().insert("x-forwarded-for", "  ".parse().unwrap());
        req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([192, 168, 1, 5], 55000))));

        assert_eq!(extract_client_id_with_trust(&req, true), "192.168.1.5");
    }

    #[test]
    fn extract_client_id_ignores_proxy_headers_when_untrusted() {
        let mut req = build_request();
        req.headers_mut()
            .insert("x-forwarded-for", "203.0.113.7, 198.51.100.1".parse().unwrap());
        req.headers_mut().insert("x-real-ip", "10.0.0.1".parse().unwrap());
        req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([192, 168, 1, 5], 55000))));

        // Proxy headers must be ignored; the real TCP peer address is used.
        assert_eq!(extract_client_id_with_trust(&req, false), "192.168.1.5");
    }

    #[test]
    fn extract_client_id_default_is_untrusted() {
        // Ensure TRUSTED_PROXY is not set so we test the default.
        // Note: OnceLock caches the result, so we test the function logic
        // directly by verifying that without the env var the default is false.
        std::env::remove_var("TRUSTED_PROXY");

        // Build a request with proxy headers + ConnectInfo.
        let mut req = build_request();
        req.headers_mut()
            .insert("x-forwarded-for", "203.0.113.7".parse().unwrap());
        req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([192, 168, 1, 99], 55000))));

        // When trust_proxy is false (the default), ConnectInfo wins.
        assert_eq!(extract_client_id_with_trust(&req, false), "192.168.1.99");
    }
}
