//! MCP (Model Context Protocol) JSON-RPC endpoint.
//!
//! Mounts the dispatch handler implemented in
//! `crate::services::mcp_server::dispatch` as a single POST route, matching
//! the MCP HTTP transport convention (one JSON-RPC endpoint per server).
//!
//! Requires authentication (`AuthUser`), matching every other document-
//! returning route in this codebase. The MCP source-scope filter (see
//! `services::mcp_server::MCP_SOURCE_SCOPE_SQL`) restricts *which
//! categories/paths* are visible over MCP, but it is not by itself an
//! authorization boundary — every document query additionally requires
//! `d.user_id = <authenticated caller>` or `d.is_public = true` (never
//! `sensitivity = 'confidential'` for non-owners), exactly like
//! `routes/documents.rs`. Without a real, non-spoofable authenticated
//! user id, an MCP client could otherwise read full content for every
//! in-scope document across every user, including private/confidential
//! ones.

use axum::{extract::State, routing::post, Json, Router};

use crate::middleware::AuthUser;
use crate::services::mcp_server::{McpRequest, McpResponse};
use crate::AppState;

async fn handle(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(request): Json<McpRequest>,
) -> Json<McpResponse> {
    Json(
        crate::services::mcp_server::dispatch(
            request,
            &state.db,
            &state.embedding_service,
            auth_user.id,
        )
        .await,
    )
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", post(handle))
}
