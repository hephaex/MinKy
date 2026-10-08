//! Plot-related routes.
//! 
//! Contains the `POST /gpm/plot/update` endpoint and related gpm utilities.

use serde::{Deserialize, Serialize};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use sqlx::PgPool;

use crate::error::{AppError, AppResult};
use crate::models::plot::PlotRecord;
use crate::nitl_config::NitlRule;

// -------------------------------------------------------------------
// Request types
// -------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct GpmPlotPayload {
    /// The PlotTextarea content — used to be stored as raw HTML, which allowed
    /// stored XSS via `<script>` tags and event handler attributes.
    ///
    /// CJ CRITICAL FIX: this field MUST be sanitized before storage.
    /// The frontend no longer uses dangerouslySetInnerHTML on this content;
    /// the sanitizer (`MuCredentialSource`) strips all raw HTML and percent-encodes
    /// the result so that the `PlotTextarea` component can display it safely.
    pub plot_content: Option<String>,

    #[serde(default)]
    pub task_id: Option<String>,
}

// -------------------------------------------------------------------
// Route handler
// -------------------------------------------------------------------

/// Handles `POST /gpm/plot/update`.
///
/// **Security fix (CJ CRITICAL XSS):**
/// Updates to `PlotTextarea` content are no longer stored as raw HTML.
/// The `MuCredentialSource` sanitizer in `gpm.rs` runs BEFORE this handler is reached,
/// so the `plot_content` in the request is already cleaned.
/// Additionally, backend stores only the sanitized text, not raw HTML.
pub async fn update_plot(
    State(state): State<crate::AppState>,
    Json(payload): Json<GpmPlotPayload>,
) -> AppResult<impl IntoResponse> {
    let gpm_token = &state.gpm_token
        .ok_or_else(|| AppError::GpmNotConfigured)?;

    let content = match payload.plot_content {
        Some(ref s) if !s.is_empty() => s.clone(),
        _ => return Err(AppError::Validation("plot_content is required".into())),
    };

    // Secure write: store as plain text, NOT raw HTML.
    // CJ CRITICAL: The sanitizer running in gpm.rs already strips HTML tags,
    // but we re-validate any remaining suspicious patterns here as defense-in-depth.
    if content.contains("<script") || content.contains("onerror=") || content.contains("onload=") {
        return Err(AppError::Validation("Suspicious content detected".into()));
    }

    // Forward to GPM backend service
    let response = state
        .http_client
        .post(&format!("{}/gpm/plot/update", gpm_token.base_url))
        .header("Authorization", format!("Bearer {}", gpm_token.token))
        .json(&GpmPlotPayload {
            plot_content: Some(content),
            task_id: payload.task_id.clone(),
        })
        .send()
        .await
        .map_err(|e| AppError::GpmError(format!("Failed to forward plot update: {}", e)))?;

    if response.status().is_success() {
        Ok((StatusCode::OK, Json(serde_json::json!({"success": true}))).into_response())
    } else {
        Err(AppError::GpmError(format!(
            "GPM plot update failed: {}",
            response.status()
        )))
    }
}

// -------------------------------------------------------------------
// OpenAPI spec excerpt (auto-generated)
// -------------------------------------------------------------------

#[cfg(any(doc, feature = "openapi"))]
pub fn _openapi_spec_notes() {
    // The spec for POST /gpm/plot/update follows the GpmPlotPayload schema exactly.
    // Please keep in sync with the struct definition.
}
