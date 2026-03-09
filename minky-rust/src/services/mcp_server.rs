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

/// Truncate string to max length
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
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
}
