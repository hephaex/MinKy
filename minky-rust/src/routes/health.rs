use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/ready", get(readiness_check))
        .route("/health/live", get(liveness_check))
}

/// Service status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Individual service check result
#[derive(Debug, Clone, Serialize)]
pub struct ServiceCheck {
    pub status: ServiceStatus,
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Complete health response
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: ServiceStatus,
    pub version: String,
    pub uptime_secs: u64,
    pub checks: HealthChecks,
}

/// All service checks
#[derive(Debug, Clone, Serialize)]
pub struct HealthChecks {
    pub database: ServiceCheck,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redis: Option<ServiceCheck>,
}

/// Check database health
async fn check_database(state: &AppState) -> ServiceCheck {
    let start = Instant::now();

    match sqlx::query("SELECT 1").fetch_one(&state.db).await {
        Ok(_) => {
            let pool = &state.db;
            let pool_size = pool.size();
            let idle_connections = pool.num_idle();

            ServiceCheck {
                status: ServiceStatus::Healthy,
                latency_ms: start.elapsed().as_millis() as u64,
                message: None,
                details: Some(serde_json::json!({
                    "pool_size": pool_size,
                    "idle_connections": idle_connections,
                })),
            }
        }
        Err(e) => ServiceCheck {
            status: ServiceStatus::Unhealthy,
            latency_ms: start.elapsed().as_millis() as u64,
            message: Some(format!("Database connection failed: {}", e)),
            details: None,
        },
    }
}

/// Check Redis health (if configured)
async fn check_redis() -> Option<ServiceCheck> {
    let redis_url = match std::env::var("REDIS_URL") {
        Ok(url) => url,
        Err(_) => return None, // Redis not configured
    };

    let start = Instant::now();

    match redis::Client::open(redis_url.as_str()) {
        Ok(client) => match client.get_multiplexed_async_connection().await {
            Ok(mut conn) => {
                let result: Result<String, redis::RedisError> =
                    redis::cmd("PING").query_async(&mut conn).await;

                match result {
                    Ok(response) if response == "PONG" => Some(ServiceCheck {
                        status: ServiceStatus::Healthy,
                        latency_ms: start.elapsed().as_millis() as u64,
                        message: None,
                        details: None,
                    }),
                    Ok(response) => Some(ServiceCheck {
                        status: ServiceStatus::Degraded,
                        latency_ms: start.elapsed().as_millis() as u64,
                        message: Some(format!("Unexpected PING response: {}", response)),
                        details: None,
                    }),
                    Err(e) => Some(ServiceCheck {
                        status: ServiceStatus::Unhealthy,
                        latency_ms: start.elapsed().as_millis() as u64,
                        message: Some(format!("Redis PING failed: {}", e)),
                        details: None,
                    }),
                }
            }
            Err(e) => Some(ServiceCheck {
                status: ServiceStatus::Unhealthy,
                latency_ms: start.elapsed().as_millis() as u64,
                message: Some(format!("Redis connection failed: {}", e)),
                details: None,
            }),
        },
        Err(e) => Some(ServiceCheck {
            status: ServiceStatus::Unhealthy,
            latency_ms: start.elapsed().as_millis() as u64,
            message: Some(format!("Invalid Redis URL: {}", e)),
            details: None,
        }),
    }
}

/// Calculate overall status from individual checks
fn calculate_overall_status(checks: &HealthChecks) -> ServiceStatus {
    // If database is unhealthy, system is unhealthy
    if checks.database.status == ServiceStatus::Unhealthy {
        return ServiceStatus::Unhealthy;
    }

    // If Redis is configured and unhealthy, system is degraded
    if let Some(ref redis) = checks.redis {
        if redis.status == ServiceStatus::Unhealthy {
            return ServiceStatus::Degraded;
        }
    }

    // If any check is degraded, system is degraded
    if checks.database.status == ServiceStatus::Degraded {
        return ServiceStatus::Degraded;
    }

    if let Some(ref redis) = checks.redis {
        if redis.status == ServiceStatus::Degraded {
            return ServiceStatus::Degraded;
        }
    }

    ServiceStatus::Healthy
}

/// Application start time (for uptime calculation)
static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn get_uptime_secs() -> u64 {
    START_TIME
        .get_or_init(Instant::now)
        .elapsed()
        .as_secs()
}

/// Comprehensive health check endpoint
async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_check = check_database(&state).await;
    let redis_check = check_redis().await;

    let checks = HealthChecks {
        database: db_check,
        redis: redis_check,
    };

    let overall_status = calculate_overall_status(&checks);

    let response = HealthResponse {
        status: overall_status.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: get_uptime_secs(),
        checks,
    };

    let status_code = match overall_status {
        ServiceStatus::Healthy => StatusCode::OK,
        ServiceStatus::Degraded => StatusCode::OK, // Still serving, just degraded
        ServiceStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(response))
}

/// Kubernetes readiness probe - is the service ready to accept traffic?
async fn readiness_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_check = check_database(&state).await;

    if db_check.status == ServiceStatus::Healthy {
        (StatusCode::OK, Json(serde_json::json!({ "ready": true })))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "ready": false,
                "reason": db_check.message
            })),
        )
    }
}

/// Kubernetes liveness probe - is the service alive?
async fn liveness_check() -> impl IntoResponse {
    // Liveness just checks if the process is running
    (StatusCode::OK, Json(serde_json::json!({ "alive": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_status_serialization() {
        assert_eq!(
            serde_json::to_string(&ServiceStatus::Healthy).unwrap(),
            "\"healthy\""
        );
        assert_eq!(
            serde_json::to_string(&ServiceStatus::Degraded).unwrap(),
            "\"degraded\""
        );
        assert_eq!(
            serde_json::to_string(&ServiceStatus::Unhealthy).unwrap(),
            "\"unhealthy\""
        );
    }

    #[test]
    fn test_calculate_overall_status_all_healthy() {
        let checks = HealthChecks {
            database: ServiceCheck {
                status: ServiceStatus::Healthy,
                latency_ms: 5,
                message: None,
                details: None,
            },
            redis: Some(ServiceCheck {
                status: ServiceStatus::Healthy,
                latency_ms: 2,
                message: None,
                details: None,
            }),
        };
        assert_eq!(calculate_overall_status(&checks), ServiceStatus::Healthy);
    }

    #[test]
    fn test_calculate_overall_status_db_unhealthy() {
        let checks = HealthChecks {
            database: ServiceCheck {
                status: ServiceStatus::Unhealthy,
                latency_ms: 0,
                message: Some("Connection failed".to_string()),
                details: None,
            },
            redis: Some(ServiceCheck {
                status: ServiceStatus::Healthy,
                latency_ms: 2,
                message: None,
                details: None,
            }),
        };
        assert_eq!(calculate_overall_status(&checks), ServiceStatus::Unhealthy);
    }

    #[test]
    fn test_calculate_overall_status_redis_unhealthy() {
        let checks = HealthChecks {
            database: ServiceCheck {
                status: ServiceStatus::Healthy,
                latency_ms: 5,
                message: None,
                details: None,
            },
            redis: Some(ServiceCheck {
                status: ServiceStatus::Unhealthy,
                latency_ms: 0,
                message: Some("Redis down".to_string()),
                details: None,
            }),
        };
        // Redis failure should be degraded, not unhealthy
        assert_eq!(calculate_overall_status(&checks), ServiceStatus::Degraded);
    }

    #[test]
    fn test_calculate_overall_status_no_redis() {
        let checks = HealthChecks {
            database: ServiceCheck {
                status: ServiceStatus::Healthy,
                latency_ms: 5,
                message: None,
                details: None,
            },
            redis: None,
        };
        assert_eq!(calculate_overall_status(&checks), ServiceStatus::Healthy);
    }

    #[test]
    fn test_get_uptime_secs_returns_reasonable_value() {
        let uptime = get_uptime_secs();
        // Should be a small positive number since we just started
        assert!(uptime < 3600); // Less than an hour
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: ServiceStatus::Healthy,
            version: "0.1.0".to_string(),
            uptime_secs: 120,
            checks: HealthChecks {
                database: ServiceCheck {
                    status: ServiceStatus::Healthy,
                    latency_ms: 5,
                    message: None,
                    details: Some(serde_json::json!({"pool_size": 10})),
                },
                redis: None,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"version\":\"0.1.0\""));
        assert!(json.contains("\"uptime_secs\":120"));
    }
}
