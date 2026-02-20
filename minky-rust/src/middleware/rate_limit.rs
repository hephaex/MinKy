use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
    Json,
};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// Rate limiter state
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    pub async fn check(&self, key: &str) -> bool {
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

    /// Cleanup old entries periodically
    pub async fn cleanup(&self) {
        let now = Instant::now();
        let mut requests = self.requests.write().await;
        requests.retain(|_, times| {
            times.retain(|&time| now.duration_since(time) < self.window);
            !times.is_empty()
        });
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Extract client identifier (IP or user ID)
    let client_id = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Use a simple in-memory rate limiter (100 requests per minute)
    static LIMITER: std::sync::OnceLock<RateLimiter> = std::sync::OnceLock::new();
    let limiter = LIMITER.get_or_init(|| RateLimiter::new(100, 60));

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
    async fn test_check_allows_requests_under_limit() {
        let limiter = RateLimiter::new(3, 60);
        assert!(limiter.check("client1").await);
        assert!(limiter.check("client1").await);
        assert!(limiter.check("client1").await);
    }

    #[tokio::test]
    async fn test_check_blocks_request_at_limit() {
        let limiter = RateLimiter::new(2, 60);
        assert!(limiter.check("client2").await);
        assert!(limiter.check("client2").await);
        // Third request should be blocked
        assert!(!limiter.check("client2").await);
    }

    #[tokio::test]
    async fn test_check_different_keys_are_independent() {
        let limiter = RateLimiter::new(1, 60);
        assert!(limiter.check("client_a").await);
        // client_a is now at limit, but client_b should still be allowed
        assert!(!limiter.check("client_a").await);
        assert!(limiter.check("client_b").await);
    }

    #[tokio::test]
    async fn test_cleanup_does_not_panic_on_empty_state() {
        let limiter = RateLimiter::new(10, 60);
        // cleanup on empty state should be a no-op
        limiter.cleanup().await;
    }

    #[tokio::test]
    async fn test_cleanup_removes_all_entries_after_short_window() {
        let limiter = RateLimiter::new(10, 0); // 0-second window expires instantly
        limiter.check("cleanup_client").await;
        // After window=0 all timestamps are immediately expired
        limiter.cleanup().await;
        // Now the key should have been removed; a fresh check should succeed
        assert!(limiter.check("cleanup_client").await);
    }
}
