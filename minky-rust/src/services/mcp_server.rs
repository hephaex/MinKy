//! MCP (Model Context Protocol) Server
//!
//! Exposes MinKy search capabilities via MCP for Claude Desktop integration.
//! Inspired by QMD's MCP server pattern.
//!
//! # Tools Exposed
//! - `minky_search` - Keyword search
//! - `minky_vsearch` - Vector semantic search
//! - `minky_deep_search` - Full hybrid search with re-ranking
//! - `minky_get` - Get document by ID or path
//! - `minky_multi_get` - Batch document retrieval
//! - `minky_collections` - List all collections with their context

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// MCP Protocol Types
// ---------------------------------------------------------------------------

/// MCP Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
    /// Protocol version
    pub protocol_version: String,
    /// Capabilities
    pub capabilities: McpCapabilities,
}

impl Default for McpServerInfo {
    fn default() -> Self {
        Self {
            name: "minky".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: "2024-11-05".to_string(),
            capabilities: McpCapabilities::default(),
        }
    }
}

/// MCP Server capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpCapabilities {
    /// Tools capability
    pub tools: Option<McpToolsCapability>,
    /// Resources capability
    pub resources: Option<McpResourcesCapability>,
    /// Prompts capability
    pub prompts: Option<McpPromptsCapability>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpToolsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpResourcesCapability {
    pub subscribe: bool,
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpPromptsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

// ---------------------------------------------------------------------------
// MCP Tool Definitions
// ---------------------------------------------------------------------------

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema (JSON Schema)
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// Get all MinKy MCP tools
pub fn get_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "minky_search".to_string(),
            description: "Search documents using keyword matching (BM25). Fast, good for exact terms.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results (default: 10)",
                        "default": 10
                    },
                    "collection": {
                        "type": "string",
                        "description": "Collection to search in (optional)"
                    }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "minky_vsearch".to_string(),
            description: "Search documents using semantic similarity. Good for conceptual queries.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results (default: 10)",
                        "default": 10
                    },
                    "threshold": {
                        "type": "number",
                        "description": "Minimum similarity (0.0-1.0, default: 0.5)",
                        "default": 0.5
                    }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "minky_deep_search".to_string(),
            description: "Full hybrid search combining keyword, semantic, and LLM re-ranking. Most accurate but slower.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results (default: 10)",
                        "default": 10
                    },
                    "expand_query": {
                        "type": "boolean",
                        "description": "Generate query alternatives (default: true)",
                        "default": true
                    },
                    "rerank": {
                        "type": "boolean",
                        "description": "Use LLM re-ranking (default: true)",
                        "default": true
                    }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "minky_get".to_string(),
            description: "Get a document by ID or path.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Document ID (UUID) or path"
                    },
                    "include_embeddings": {
                        "type": "boolean",
                        "description": "Include vector embeddings (default: false)",
                        "default": false
                    }
                },
                "required": ["id"]
            }),
        },
        McpTool {
            name: "minky_multi_get".to_string(),
            description: "Get multiple documents by IDs or glob pattern.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Document IDs or paths"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g., 'notes/*.md')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max documents (default: 20)",
                        "default": 20
                    }
                }
            }),
        },
        McpTool {
            name: "minky_collections".to_string(),
            description: "List all collections with their context.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

// ---------------------------------------------------------------------------
// MCP Request/Response Types
// ---------------------------------------------------------------------------

/// MCP JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID
    pub id: serde_json::Value,
    /// Method name
    pub method: String,
    /// Parameters
    #[serde(default)]
    pub params: serde_json::Value,
}

/// MCP JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID
    pub id: serde_json::Value,
    /// Result (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

/// MCP Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl McpResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: serde_json::Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(McpError {
                code,
                message,
                data: None,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Source-scope filtering
// ---------------------------------------------------------------------------

/// SQL fragment (as a standalone `WHERE`-compatible boolean expression)
/// implementing the MCP source-scope filter defined by the C4 plan, AND the
/// same ownership/visibility rule enforced by every other authenticated
/// document route in this codebase (see e.g. `routes/documents.rs`'s
/// `(d.user_id = $N OR d.is_public = true)` pattern).
///
/// A document is visible over MCP only if:
/// 1. it is either
///    (a) categorized under the migration-015 `memory-*` gyeol category tree
///        (resolved dynamically by slug — category IDs are NOT hardcoded,
///        since migration 015 inserts them via `ON CONFLICT (slug)` without
///        a fixed ID), or
///    (b) has a `source_path` under the references-root prefix,
/// **AND**
/// 2. the caller either owns the document (`d.user_id = $USERID_PARAM`) or
///    the document is public (`d.is_public = true`) — never confidential
///    (`d.sensitivity <> 'confidential'` unless the caller is the owner).
///
/// The MCP route requires authentication (`AuthUser`) precisely so this
/// ownership/visibility check has a real, non-spoofable `$USERID_PARAM` to
/// bind — the JSON-RPC layer never accepts a caller-supplied user id.
///
/// Everything else (documents with neither a memory-* category nor a
/// references-root source_path, or private/confidential documents owned by
/// someone else) is excluded from MCP visibility.
///
/// The references-root prefix is intentionally NOT hardcoded here: the C0
/// goal (which establishes the concrete ingestion convention for the
/// references corpus) is blocked on an unresolved corpus/instance
/// question (see the durable ultragoal ledger). Until C0 resolves that,
/// this reads the prefix from `MINKY_MCP_REFERENCES_ROOT_PREFIX` with a
/// conservative empty-string default that matches nothing, so the
/// source_path half of the filter fails safe (excludes everything) rather
/// than guessing a prefix and silently leaking out-of-scope documents.
const MCP_SOURCE_SCOPE_SQL: &str = r#"
    (
        d.category_id IN (
            SELECT c.id FROM categories c
            WHERE c.slug IN (
                'memory-episode', 'memory-semantic', 'memory-bond',
                'memory-reflection', 'memory-reference'
            )
        )
        OR (
            $REFROOT_PARAM::text <> ''
            AND d.source_path IS NOT NULL
            AND d.source_path LIKE $REFROOT_PARAM::text || '%'
        )
    )
    AND (
        d.user_id = $USERID_PARAM
        OR (d.is_public = true AND d.sensitivity <> 'confidential')
    )
"#;

/// Read the references-root source_path prefix used by the MCP source-scope
/// filter. Empty string (the default) makes the source_path half of the
/// filter match nothing, so only memory-* categorized documents are visible
/// until C0 establishes and configures the real prefix.
fn references_root_prefix() -> String {
    std::env::var("MINKY_MCP_REFERENCES_ROOT_PREFIX").unwrap_or_default()
}

/// Row shape for a source-scope-filtered document fetch (minky_get /
/// minky_multi_get).
#[derive(sqlx::FromRow)]
#[allow(dead_code)] // category_id/created_at/updated_at document the full scoped-row shape; not all fields are rendered in every tool's text output yet
struct ScopedDocumentRow {
    id: uuid::Uuid,
    title: String,
    content: String,
    category_id: Option<i32>,
    source_path: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// Dispatch a single MCP JSON-RPC request against `pool`/`embedding_service`,
/// enforcing the source-scope filter on every document-returning tool.
///
/// Supported methods:
/// - `initialize` — returns server info + capabilities.
/// - `tools/list` — returns [`get_tools`].
/// - `tools/call` — dispatches to one of the six `minky_*` tools by
///   `params.name`, with `params.arguments` deserialized into the
///   corresponding `*Input` struct.
///
/// Unknown methods return a JSON-RPC `-32601 Method not found` error;
/// malformed params return `-32602 Invalid params`; internal failures
/// return `-32603 Internal error` with the failure reason in `message`.
pub async fn dispatch(
    request: McpRequest,
    pool: &sqlx::PgPool,
    embedding_service: &std::sync::Arc<crate::services::EmbeddingService>,
    user_id: i32,
) -> McpResponse {
    match request.method.as_str() {
        "initialize" => McpResponse::success(
            request.id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": McpServerInfo::default(),
                "capabilities": McpCapabilities {
                    tools: Some(McpToolsCapability::default()),
                    resources: None,
                    prompts: None,
                },
            }),
        ),
        "tools/list" => McpResponse::success(
            request.id,
            serde_json::json!({ "tools": get_tools() }),
        ),
        "tools/call" => {
            let tool_name = match request.params.get("name").and_then(|v| v.as_str()) {
                Some(name) => name.to_string(),
                None => {
                    return McpResponse::error(
                        request.id,
                        -32602,
                        "Invalid params: missing 'name'".to_string(),
                    );
                }
            };
            let arguments = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            let result = dispatch_tool_call(&tool_name, arguments, pool, embedding_service, user_id).await;
            match result {
                Ok(tool_result) => McpResponse::success(
                    request.id,
                    serde_json::to_value(tool_result).unwrap_or_default(),
                ),
                Err(McpDispatchError::InvalidParams(msg)) => McpResponse::error(request.id, -32602, msg),
                Err(McpDispatchError::Internal(msg)) => McpResponse::error(request.id, -32603, msg),
            }
        }
        other => McpResponse::error(
            request.id,
            -32601,
            format!("Method not found: {}", other),
        ),
    }
}

/// Distinguishes client-caused (`-32602 Invalid params`) from
/// server-caused (`-32603 Internal error`) failures inside
/// `dispatch_tool_call`, so a malformed `arguments` payload from an MCP
/// client is never mislabeled as a server fault.
enum McpDispatchError {
    InvalidParams(String),
    Internal(String),
}

impl From<serde_json::Error> for McpDispatchError {
    fn from(e: serde_json::Error) -> Self {
        McpDispatchError::InvalidParams(format!("Invalid params: {}", e))
    }
}

impl From<sqlx::Error> for McpDispatchError {
    fn from(e: sqlx::Error) -> Self {
        McpDispatchError::Internal(e.to_string())
    }
}

impl From<anyhow::Error> for McpDispatchError {
    fn from(e: anyhow::Error) -> Self {
        McpDispatchError::Internal(e.to_string())
    }
}

/// Maximum `limit` any MCP tool will honor, regardless of what a client
/// requests, to bound how much full document content a single authenticated
/// call can pull in one round-trip.
const MCP_MAX_LIMIT: usize = 100;

/// Multiplier applied when over-fetching hybrid search results before
/// scope-filtering, so a requested `limit` of in-scope results is still
/// satisfied even though the MCP-visible corpus is typically a small
/// minority of the full multi-user document store. This is a bounded
/// heuristic, not a guarantee for pathologically sparse corpora — see the
/// module-level docs on `filter_scoped_results`.
const MCP_SEARCH_OVERFETCH_FACTOR: usize = 5;

async fn dispatch_tool_call(
    tool_name: &str,
    arguments: serde_json::Value,
    pool: &sqlx::PgPool,
    embedding_service: &std::sync::Arc<crate::services::EmbeddingService>,
    user_id: i32,
) -> Result<McpToolResult, McpDispatchError> {
    match tool_name {
        "minky_search" | "minky_vsearch" | "minky_deep_search" => {
            let (query, requested_limit, mode, threshold, expand_query, rerank) = match tool_name {
                "minky_search" => {
                    let input: SearchInput = serde_json::from_value(arguments)?;
                    if input.collection.is_some() {
                        return Err(McpDispatchError::InvalidParams(
                            "Invalid params: 'collection' filtering is not yet implemented for minky_search".to_string(),
                        ));
                    }
                    (input.query, input.limit, crate::services::hybrid_search::SearchMode::Keyword, 0.0, false, false)
                }
                "minky_vsearch" => {
                    let input: VSearchInput = serde_json::from_value(arguments)?;
                    (input.query, input.limit, crate::services::hybrid_search::SearchMode::Vector, input.threshold, false, false)
                }
                _ => {
                    let input: DeepSearchInput = serde_json::from_value(arguments)?;
                    (input.query, input.limit, crate::services::hybrid_search::SearchMode::Deep, 0.2, input.expand_query, input.rerank)
                }
            };
            let capped_limit = requested_limit.min(MCP_MAX_LIMIT);
            let hybrid = crate::services::hybrid_search::HybridSearchService::new(
                pool.clone(),
                embedding_service.clone(),
            );
            // Over-fetch before scope-filtering: the MCP-visible corpus
            // (gyeol memory-* + references) is typically a small minority
            // of the full multi-user document store, so filtering AFTER a
            // tight `limit` window would frequently starve results even
            // when many in-scope documents exist deeper in the ranking.
            let response = hybrid
                .search(crate::services::hybrid_search::HybridSearchRequest {
                    query,
                    mode,
                    limit: capped_limit.saturating_mul(MCP_SEARCH_OVERFETCH_FACTOR),
                    threshold,
                    expand_query,
                    rerank,
                    collection_id: None,
                    user_id: None,
                })
                .await
                .map_err(|e| McpDispatchError::Internal(e.to_string()))?;
            let mut scoped = filter_scoped_results(pool, response.results, user_id).await?;
            scoped.truncate(capped_limit);
            Ok(format_search_results(&scoped, OutputFormat::Text))
        }
        "minky_get" => {
            let input: GetInput = serde_json::from_value(arguments)?;
            let refroot = references_root_prefix();
            let doc = if let Ok(id) = uuid::Uuid::parse_str(&input.id) {
                let sql = format!(
                    "SELECT d.id, d.title, d.content, d.category_id, d.source_path, d.created_at, d.updated_at \
                     FROM documents d WHERE d.id = $1 AND d.archived = false AND {}",
                    MCP_SOURCE_SCOPE_SQL
                        .replace("$REFROOT_PARAM", "$2")
                        .replace("$USERID_PARAM", "$3")
                );
                sqlx::query_as::<_, ScopedDocumentRow>(&sql)
                    .bind(id)
                    .bind(&refroot)
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?
            } else {
                let sql = format!(
                    "SELECT d.id, d.title, d.content, d.category_id, d.source_path, d.created_at, d.updated_at \
                     FROM documents d WHERE d.source_path = $1 AND d.archived = false AND {}",
                    MCP_SOURCE_SCOPE_SQL
                        .replace("$REFROOT_PARAM", "$2")
                        .replace("$USERID_PARAM", "$3")
                );
                sqlx::query_as::<_, ScopedDocumentRow>(&sql)
                    .bind(&input.id)
                    .bind(&refroot)
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?
            };
            // `include_embeddings` is accepted (schema-advertised) but has no
            // effect: minky_get renders document text only, never raw
            // embedding vectors, so there is nothing for this flag to turn
            // on or off. Documented here rather than silently accepted with
            // no signal to the caller.
            let _ = input.include_embeddings;
            match doc {
                Some(d) => Ok(McpToolResult {
                    content: vec![McpContent::Text {
                        text: format!(
                            "# {}\n\nID: {}\nSource: {}\n\n{}",
                            d.title,
                            d.id,
                            d.source_path.unwrap_or_else(|| "(none)".to_string()),
                            d.content
                        ),
                    }],
                    is_error: false,
                }),
                None => Ok(McpToolResult {
                    content: vec![McpContent::Text {
                        text: format!(
                            "Document '{}' not found or not visible to MCP (outside the gyeol memory / references source scope, or not owned/public).",
                            input.id
                        ),
                    }],
                    is_error: true,
                }),
            }
        }
        "minky_multi_get" => {
            let input: MultiGetInput = serde_json::from_value(arguments)?;
            if input.ids.is_empty() {
                if input.pattern.is_some() {
                    return Err(McpDispatchError::InvalidParams(
                        "Invalid params: 'pattern' glob matching is not yet implemented for minky_multi_get; pass explicit 'ids' instead".to_string(),
                    ));
                }
                return Err(McpDispatchError::InvalidParams(
                    "Invalid params: minky_multi_get requires a non-empty 'ids' array".to_string(),
                ));
            }
            let refroot = references_root_prefix();
            let capped_limit = input.limit.min(MCP_MAX_LIMIT);
            let ids: Vec<uuid::Uuid> = input
                .ids
                .iter()
                .filter_map(|s| uuid::Uuid::parse_str(s).ok())
                .collect();
            let sql = format!(
                "SELECT d.id, d.title, d.content, d.category_id, d.source_path, d.created_at, d.updated_at \
                 FROM documents d WHERE d.id = ANY($1) AND d.archived = false AND {} \
                 LIMIT $4",
                MCP_SOURCE_SCOPE_SQL
                    .replace("$REFROOT_PARAM", "$2")
                    .replace("$USERID_PARAM", "$3")
            );
            let docs = sqlx::query_as::<_, ScopedDocumentRow>(&sql)
                .bind(&ids)
                .bind(&refroot)
                .bind(user_id)
                .bind(capped_limit as i64)
                .fetch_all(pool)
                .await?;
            let text = docs
                .iter()
                .map(|d| format!("## {} ({})\n\n{}", d.title, d.id, truncate(&d.content, 300)))
                .collect::<Vec<_>>()
                .join("\n\n---\n\n");
            Ok(McpToolResult {
                content: vec![McpContent::Text {
                    text: if text.is_empty() {
                        "No documents found in the requested set within MCP source scope.".to_string()
                    } else {
                        text
                    },
                }],
                is_error: false,
            })
        }
        "minky_collections" => {
            // Not scoped by MCP_SOURCE_SCOPE_SQL: collections are a distinct
            // concept from documents (id/name/description metadata only, no
            // document content), and the C4 plan's source-scope requirement
            // is stated in terms of document visibility. Still requires
            // authentication at the route level like every other tool.
            let rows: Vec<(uuid::Uuid, String, Option<String>)> = sqlx::query_as(
                "SELECT id, name, description FROM collections ORDER BY name",
            )
            .fetch_all(pool)
            .await?;
            let text = rows
                .iter()
                .map(|(id, name, desc)| {
                    format!("- {} ({}){}", name, id, desc.as_deref().map(|d| format!(": {}", d)).unwrap_or_default())
                })
                .collect::<Vec<_>>()
                .join("\n");
            Ok(McpToolResult {
                content: vec![McpContent::Text {
                    text: if text.is_empty() {
                        "No collections found.".to_string()
                    } else {
                        text
                    },
                }],
                is_error: false,
            })
        }
        other => Ok(McpToolResult {
            content: vec![McpContent::Text {
                text: format!("Unknown tool: {}", other),
            }],
            is_error: true,
        }),
    }
}

/// Re-filter hybrid search results against the MCP source-scope predicate
/// AND the caller's ownership/visibility (see `MCP_SOURCE_SCOPE_SQL`).
/// `HybridSearchService::search` queries across the whole `documents` table
/// (no source-scope or per-caller ownership awareness), so results are
/// post-filtered here by re-checking each matched document_id against the
/// same scope query used by `minky_get`/`minky_multi_get`, preserving the
/// search engine's relevance ranking (order-preserving filter, not a
/// re-sort). Callers should over-fetch before calling this (see
/// `MCP_SEARCH_OVERFETCH_FACTOR`) since the MCP-visible corpus is typically
/// a small minority of the full document store.
async fn filter_scoped_results(
    pool: &sqlx::PgPool,
    results: Vec<crate::services::hybrid_search::HybridSearchResult>,
    user_id: i32,
) -> Result<Vec<crate::services::hybrid_search::HybridSearchResult>, McpDispatchError> {
    if results.is_empty() {
        return Ok(results);
    }
    let refroot = references_root_prefix();
    let ids: Vec<uuid::Uuid> = results.iter().map(|r| r.document_id).collect();
    let sql = format!(
        "SELECT d.id FROM documents d WHERE d.id = ANY($1) AND d.archived = false AND {}",
        MCP_SOURCE_SCOPE_SQL
            .replace("$REFROOT_PARAM", "$2")
            .replace("$USERID_PARAM", "$3")
    );
    let visible_ids: std::collections::HashSet<uuid::Uuid> = sqlx::query_scalar(&sql)
        .bind(&ids)
        .bind(&refroot)
        .bind(user_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect();
    Ok(results
        .into_iter()
        .filter(|r| visible_ids.contains(&r.document_id))
        .collect())
}


// ---------------------------------------------------------------------------
// MCP Tool Call Types
// ---------------------------------------------------------------------------

/// Tool call result content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    /// Content items
    pub content: Vec<McpContent>,
    /// Whether the result is an error
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

/// MCP Content item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        text: String,
    },
    /// Image content
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    /// Resource content
    #[serde(rename = "resource")]
    Resource {
        resource: McpResource,
    },
}

/// MCP Resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// Resource URI
    pub uri: String,
    /// Resource name
    pub name: String,
    /// Resource description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Tool Input Types
// ---------------------------------------------------------------------------

/// Input for minky_search
#[derive(Debug, Clone, Deserialize)]
pub struct SearchInput {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub collection: Option<String>,
}

/// Input for minky_vsearch
#[derive(Debug, Clone, Deserialize)]
pub struct VSearchInput {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default = "default_threshold")]
    pub threshold: f32,
}

/// Input for minky_deep_search
#[derive(Debug, Clone, Deserialize)]
pub struct DeepSearchInput {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default = "default_true")]
    pub expand_query: bool,
    #[serde(default = "default_true")]
    pub rerank: bool,
}

/// Input for minky_get
#[derive(Debug, Clone, Deserialize)]
pub struct GetInput {
    pub id: String,
    #[serde(default)]
    pub include_embeddings: bool,
}

/// Input for minky_multi_get
#[derive(Debug, Clone, Deserialize)]
pub struct MultiGetInput {
    #[serde(default)]
    pub ids: Vec<String>,
    pub pattern: Option<String>,
    #[serde(default = "default_multi_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 10 }
fn default_threshold() -> f32 { 0.5 }
fn default_true() -> bool { true }
fn default_multi_limit() -> usize { 20 }

// ---------------------------------------------------------------------------
// MCP Result Formatting
// ---------------------------------------------------------------------------

/// Format search results as MCP text content
pub fn format_search_results(
    results: &[crate::services::hybrid_search::HybridSearchResult],
    format: OutputFormat,
) -> McpToolResult {
    let text = match format {
        OutputFormat::Text => {
            results.iter().enumerate().map(|(i, r)| {
                format!(
                    "{}. {} (score: {:.2})\n   {}\n   ID: {}",
                    i + 1,
                    r.title,
                    r.score,
                    truncate(&r.snippet, 150),
                    r.document_id
                )
            }).collect::<Vec<_>>().join("\n\n")
        }
        OutputFormat::Json => {
            serde_json::to_string_pretty(results).unwrap_or_default()
        }
        OutputFormat::Markdown => {
            results.iter().enumerate().map(|(i, r)| {
                format!(
                    "### {}. {}\n**Score:** {:.2} | **ID:** `{}`\n\n{}",
                    i + 1,
                    r.title,
                    r.score,
                    r.document_id,
                    r.snippet
                )
            }).collect::<Vec<_>>().join("\n\n---\n\n")
        }
    };

    McpToolResult {
        content: vec![McpContent::Text { text }],
        is_error: false,
    }
}

/// Output format for MCP results
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Markdown,
}

/// Truncate string to at most `max` characters (not bytes), appending "..."
/// when truncated. Counts Unicode scalar values via `char_indices`, not
/// bytes, so multi-byte content (e.g. Korean, 3 bytes/char in UTF-8) is
/// never sliced mid-character — a byte-index slice like `&s[..max]` panics
/// with "byte index N is not a char boundary" whenever `max` lands inside a
/// multi-byte codepoint, which is the common case for this Korean-first
/// product's document content.
fn truncate(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((byte_idx, _)) => format!("{}...", &s[..byte_idx]),
        None => s.to_string(),
    }
}

// ---------------------------------------------------------------------------
// MCP Server Configuration
// ---------------------------------------------------------------------------

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Server name
    pub name: String,
    /// HTTP port (for HTTP transport)
    pub port: Option<u16>,
    /// Enable stdio transport
    pub stdio: bool,
    /// Enable HTTP transport
    pub http: bool,
    /// Default output format
    pub format: String,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            name: "minky".to_string(),
            port: Some(3001),
            stdio: true,
            http: false,
            format: "text".to_string(),
        }
    }
}

/// Generate Claude Desktop config JSON
pub fn generate_claude_config(config: &McpConfig) -> serde_json::Value {
    if config.http {
        serde_json::json!({
            "mcpServers": {
                &config.name: {
                    "url": format!("http://localhost:{}/mcp", config.port.unwrap_or(3001))
                }
            }
        })
    } else {
        serde_json::json!({
            "mcpServers": {
                &config.name: {
                    "command": "minky",
                    "args": ["mcp"]
                }
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_server_info_default() {
        let info = McpServerInfo::default();
        assert_eq!(info.name, "minky");
        assert_eq!(info.protocol_version, "2024-11-05");
    }

    #[test]
    fn test_get_tools() {
        let tools = get_tools();
        assert_eq!(tools.len(), 6);

        let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"minky_search"));
        assert!(names.contains(&"minky_vsearch"));
        assert!(names.contains(&"minky_deep_search"));
        assert!(names.contains(&"minky_get"));
        assert!(names.contains(&"minky_multi_get"));
        assert!(names.contains(&"minky_collections"));
    }

    #[test]
    fn test_mcp_response_success() {
        let resp = McpResponse::success(
            serde_json::json!(1),
            serde_json::json!({"key": "value"}),
        );
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_mcp_response_error() {
        let resp = McpResponse::error(
            serde_json::json!(1),
            -32600,
            "Invalid request".to_string(),
        );
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32600);
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    #[test]
    fn truncate_does_not_panic_on_multibyte_korean_content_at_odd_byte_boundary() {
        // "한국어" = 3 Korean chars, 9 UTF-8 bytes (3 bytes/char). Truncating
        // at max=5 chars must land exactly on a char boundary via
        // char_indices, never mid-codepoint the way a raw &s[..5] byte slice
        // would (which panics: "byte index 5 is not a char boundary").
        let s = "한국어 형태소 분석기 테스트 문서입니다";
        let result = truncate(s, 5);
        assert_eq!(result, "한국어 형...");
    }

    #[test]
    fn truncate_max_exactly_at_char_count_returns_unmodified() {
        assert_eq!(truncate("한국어", 3), "한국어");
    }

    #[test]
    fn test_generate_claude_config_stdio() {
        let config = McpConfig {
            stdio: true,
            http: false,
            ..Default::default()
        };
        let json = generate_claude_config(&config);
        assert!(json["mcpServers"]["minky"]["command"].is_string());
    }

    #[test]
    fn test_generate_claude_config_http() {
        let config = McpConfig {
            stdio: false,
            http: true,
            port: Some(3001),
            ..Default::default()
        };
        let json = generate_claude_config(&config);
        assert!(json["mcpServers"]["minky"]["url"].is_string());
    }

    #[test]
    fn test_mcp_content_text_serde() {
        let content = McpContent::Text { text: "hello".to_string() };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"text\""));
    }

    #[test]
    fn test_search_input_defaults() {
        let json = r#"{"query": "test"}"#;
        let input: SearchInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.limit, 10);
    }

    #[test]
    fn test_deep_search_input_defaults() {
        let json = r#"{"query": "test"}"#;
        let input: DeepSearchInput = serde_json::from_str(json).unwrap();
        assert!(input.expand_query);
        assert!(input.rerank);
    }

    #[test]
    fn references_root_prefix_defaults_to_empty_string_when_unset() {
        // SAFETY: test-only env var manipulation.
        unsafe {
            std::env::remove_var("MINKY_MCP_REFERENCES_ROOT_PREFIX");
        }
        assert_eq!(references_root_prefix(), "");
    }

    #[tokio::test]
    async fn dispatch_unknown_method_returns_json_rpc_method_not_found() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let embedding_service = std::sync::Arc::new(crate::services::EmbeddingService::new(
            pool.clone(),
            crate::services::EmbeddingConfig {
                openai_api_key: None,
                voyage_api_key: None,
                upstage_api_key: None,
                default_model: crate::models::EmbeddingModel::OpenaiTextEmbedding3Small,
                chunk_size: 512,
                chunk_overlap: 50,
                local_embedding_enabled: false,
            },
        ));
        let response = dispatch(
            McpRequest {
                jsonrpc: "2.0".to_string(),
                id: serde_json::json!(1),
                method: "not_a_real_method".to_string(),
                params: serde_json::json!({}),
            },
            &pool,
            &embedding_service,
            1,
        )
        .await;
        assert!(response.result.is_none());
        let error = response.error.expect("expected a JSON-RPC error");
        assert_eq!(error.code, -32601);
        assert!(error.message.contains("not_a_real_method"));
    }

    #[tokio::test]
    async fn dispatch_initialize_reports_all_six_tools_via_capabilities() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let embedding_service = std::sync::Arc::new(crate::services::EmbeddingService::new(
            pool.clone(),
            crate::services::EmbeddingConfig {
                openai_api_key: None,
                voyage_api_key: None,
                upstage_api_key: None,
                default_model: crate::models::EmbeddingModel::OpenaiTextEmbedding3Small,
                chunk_size: 512,
                chunk_overlap: 50,
                local_embedding_enabled: false,
            },
        ));
        let response = dispatch(
            McpRequest {
                jsonrpc: "2.0".to_string(),
                id: serde_json::json!(1),
                method: "tools/list".to_string(),
                params: serde_json::json!({}),
            },
            &pool,
            &embedding_service,
            1,
        )
        .await;
        let result = response.result.expect("tools/list must succeed");
        let tools = result["tools"].as_array().expect("tools must be an array");
        assert_eq!(tools.len(), 6, "all six minky_* tools must be reported");
    }

    #[tokio::test]
    async fn dispatch_tools_call_missing_name_returns_invalid_params() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let embedding_service = std::sync::Arc::new(crate::services::EmbeddingService::new(
            pool.clone(),
            crate::services::EmbeddingConfig {
                openai_api_key: None,
                voyage_api_key: None,
                upstage_api_key: None,
                default_model: crate::models::EmbeddingModel::OpenaiTextEmbedding3Small,
                chunk_size: 512,
                chunk_overlap: 50,
                local_embedding_enabled: false,
            },
        ));
        let response = dispatch(
            McpRequest {
                jsonrpc: "2.0".to_string(),
                id: serde_json::json!(1),
                method: "tools/call".to_string(),
                params: serde_json::json!({}),
            },
            &pool,
            &embedding_service,
            1,
        )
        .await;
        let error = response.error.expect("missing tool name must be an error");
        assert_eq!(error.code, -32602);
    }

    /// Live-DB proof of the MCP source-scope filter (C4 acceptance
    /// criteria): a document matching neither the memory-* category set
    /// nor the references source_path prefix must be excluded from
    /// `minky_get` results (negative case); a gyeol-categorized document
    /// and a references-source_path document must each be independently
    /// retrievable (positive cases).
    ///
    /// Seeds the memory-* categories directly (migration 015 is a no-op in
    /// this sandbox's dev DB because it depends on a seed user id=1 that
    /// does not exist here — a pre-existing, out-of-scope data-integrity
    /// gap unrelated to this goal) rather than depending on it having run.
    ///
    /// Requires a live Postgres with the minky schema applied. Skipped in
    /// CI; run with: cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn source_scope_filter_excludes_out_of_scope_and_includes_both_in_scope_kinds() {
        let database_url = std::env::var("MINKY_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://mare@localhost/minky_rust_db".to_string());
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("failed to connect to test database");

        // SAFETY: test-only env var manipulation; ensures the source_path
        // half of the filter is active for this test's positive case.
        unsafe {
            std::env::set_var("MINKY_MCP_REFERENCES_ROOT_PREFIX", "/mcp-test-refs/");
        }

        let user_id: i32 = sqlx::query_scalar(
            "INSERT INTO users (email, username, password_hash) \
             VALUES ($1, $2, 'x') \
             ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email \
             RETURNING id",
        )
        .bind("mcp-scope-test@minky.local")
        .bind("mcp_scope_test")
        .fetch_one(&pool)
        .await
        .expect("seed user");

        // Ensure the memory-reference category exists (migration 015 may be
        // a no-op in this sandbox; seed it directly, idempotently).
        sqlx::query(
            "INSERT INTO categories (name, user_id, slug, is_active) \
             VALUES ('reference', $1, 'memory-reference', true) \
             ON CONFLICT (slug) WHERE slug IS NOT NULL DO NOTHING",
        )
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("seed memory-reference category");

        let memory_category_id: i32 = sqlx::query_scalar(
            "SELECT id FROM categories WHERE slug = 'memory-reference'",
        )
        .fetch_one(&pool)
        .await
        .expect("memory-reference category must exist after seeding");

        // (1) In-scope via category: gyeol-categorized document.
        let gyeol_doc_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO documents (title, content, user_id, category_id) \
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind("MCP scope fixture: gyeol memory doc")
        .bind("This document is categorized under memory-reference.")
        .bind(user_id)
        .bind(memory_category_id)
        .fetch_one(&pool)
        .await
        .expect("seed gyeol-categorized document");

        // (2) In-scope via source_path: references-root document.
        let refs_doc_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO documents (title, content, user_id, source_path) \
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind("MCP scope fixture: references doc")
        .bind("This document is under the references root source_path.")
        .bind(user_id)
        .bind("/mcp-test-refs/some-doc.md")
        .fetch_one(&pool)
        .await
        .expect("seed references-source_path document");

        // (3) Out-of-scope: neither memory-* category nor references path.
        let excluded_doc_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO documents (title, content, user_id) \
             VALUES ($1, $2, $3) RETURNING id",
        )
        .bind("MCP scope fixture: unrelated doc")
        .bind("This document has no category and no references source_path.")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("seed out-of-scope document");

        let embedding_service = std::sync::Arc::new(crate::services::EmbeddingService::new(
            pool.clone(),
            crate::services::EmbeddingConfig {
                openai_api_key: None,
                voyage_api_key: None,
                upstage_api_key: None,
                default_model: crate::models::EmbeddingModel::OpenaiTextEmbedding3Small,
                chunk_size: 512,
                chunk_overlap: 50,
                local_embedding_enabled: false,
            },
        ));

        let get_via_mcp = |id: uuid::Uuid| {
            let pool = pool.clone();
            let embedding_service = embedding_service.clone();
            async move {
                dispatch(
                    McpRequest {
                        jsonrpc: "2.0".to_string(),
                        id: serde_json::json!(1),
                        method: "tools/call".to_string(),
                        params: serde_json::json!({
                            "name": "minky_get",
                            "arguments": { "id": id.to_string() }
                        }),
                    },
                    &pool,
                    &embedding_service,
                    user_id,
                )
                .await
            }
        };

        let gyeol_response = get_via_mcp(gyeol_doc_id).await;
        let refs_response = get_via_mcp(refs_doc_id).await;
        let excluded_response = get_via_mcp(excluded_doc_id).await;

        // Cleanup before asserting, so a failed assertion doesn't leave fixtures behind.
        sqlx::query("DELETE FROM documents WHERE id = ANY($1)")
            .bind(&[gyeol_doc_id, refs_doc_id, excluded_doc_id][..])
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .ok();

        // Positive case 1: gyeol-categorized document is retrievable.
        let gyeol_result = gyeol_response.result.expect("gyeol doc: MCP call must succeed");
        let gyeol_tool_result: McpToolResult = serde_json::from_value(gyeol_result).unwrap();
        assert!(
            !gyeol_tool_result.is_error,
            "gyeol-categorized document must be retrievable via minky_get: {:?}",
            gyeol_tool_result
        );

        // Positive case 2: references-source_path document is retrievable.
        let refs_result = refs_response.result.expect("refs doc: MCP call must succeed");
        let refs_tool_result: McpToolResult = serde_json::from_value(refs_result).unwrap();
        assert!(
            !refs_tool_result.is_error,
            "references-source_path document must be retrievable via minky_get: {:?}",
            refs_tool_result
        );

        // Negative case: a document matching neither predicate must be excluded.
        let excluded_result = excluded_response.result.expect("excluded doc: MCP call must succeed (returns an error-content result, not a JSON-RPC error)");
        let excluded_tool_result: McpToolResult = serde_json::from_value(excluded_result).unwrap();
        assert!(
            excluded_tool_result.is_error,
            "a document matching neither the memory-* category nor the references source_path must be excluded from MCP visibility: {:?}",
            excluded_tool_result
        );
    }

    /// Live-DB proof of the ownership/authorization boundary added in
    /// response to an architect HIGH finding: the source-scope predicate
    /// alone (category/path) is NOT sufficient to authorize MCP access — a
    /// document that satisfies the category predicate but is both private
    /// (`is_public = false`) and owned by a DIFFERENT user than the caller
    /// must still be excluded. Without this, any MCP client could read
    /// every gyeol-categorized document across every user regardless of
    /// ownership or confidentiality.
    ///
    /// Requires a live Postgres with the minky schema applied. Skipped in
    /// CI; run with: cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn source_scope_filter_excludes_private_document_owned_by_a_different_user() {
        let database_url = std::env::var("MINKY_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://mare@localhost/minky_rust_db".to_string());
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("failed to connect to test database");

        let caller_user_id: i32 = sqlx::query_scalar(
            "INSERT INTO users (email, username, password_hash) \
             VALUES ($1, $2, 'x') \
             ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email \
             RETURNING id",
        )
        .bind("mcp-auth-caller@minky.local")
        .bind("mcp_auth_caller")
        .fetch_one(&pool)
        .await
        .expect("seed caller user");

        let owner_user_id: i32 = sqlx::query_scalar(
            "INSERT INTO users (email, username, password_hash) \
             VALUES ($1, $2, 'x') \
             ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email \
             RETURNING id",
        )
        .bind("mcp-auth-owner@minky.local")
        .bind("mcp_auth_owner")
        .fetch_one(&pool)
        .await
        .expect("seed owner user");

        sqlx::query(
            "INSERT INTO categories (name, user_id, slug, is_active) \
             VALUES ('reference', $1, 'memory-reference', true) \
             ON CONFLICT (slug) WHERE slug IS NOT NULL DO NOTHING",
        )
        .bind(owner_user_id)
        .execute(&pool)
        .await
        .expect("seed memory-reference category");

        let memory_category_id: i32 =
            sqlx::query_scalar("SELECT id FROM categories WHERE slug = 'memory-reference'")
                .fetch_one(&pool)
                .await
                .expect("memory-reference category must exist after seeding");

        // Category-matching, but private and owned by a DIFFERENT user than
        // the caller — must be excluded despite satisfying the category
        // predicate.
        let private_doc_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO documents (title, content, user_id, category_id, is_public) \
             VALUES ($1, $2, $3, $4, false) RETURNING id",
        )
        .bind("MCP auth fixture: private doc owned by someone else")
        .bind("This is private content owned by a different user.")
        .bind(owner_user_id)
        .bind(memory_category_id)
        .fetch_one(&pool)
        .await
        .expect("seed private document owned by a different user");

        let embedding_service = std::sync::Arc::new(crate::services::EmbeddingService::new(
            pool.clone(),
            crate::services::EmbeddingConfig {
                openai_api_key: None,
                voyage_api_key: None,
                upstage_api_key: None,
                default_model: crate::models::EmbeddingModel::OpenaiTextEmbedding3Small,
                chunk_size: 512,
                chunk_overlap: 50,
                local_embedding_enabled: false,
            },
        ));

        let response = dispatch(
            McpRequest {
                jsonrpc: "2.0".to_string(),
                id: serde_json::json!(1),
                method: "tools/call".to_string(),
                params: serde_json::json!({
                    "name": "minky_get",
                    "arguments": { "id": private_doc_id.to_string() }
                }),
            },
            &pool,
            &embedding_service,
            caller_user_id,
        )
        .await;

        // Cleanup before asserting, so a failed assertion doesn't leave fixtures behind.
        sqlx::query("DELETE FROM documents WHERE id = $1")
            .bind(private_doc_id)
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM users WHERE id = ANY($1)")
            .bind(&[caller_user_id, owner_user_id][..])
            .execute(&pool)
            .await
            .ok();

        let result = response.result.expect("minky_get call must succeed (returns an error-content result, not a JSON-RPC error)");
        let tool_result: McpToolResult = serde_json::from_value(result).unwrap();
        assert!(
            tool_result.is_error,
            "a private document owned by a different user must be excluded from MCP visibility even though it matches the category predicate: {:?}",
            tool_result
        );
    }
}
