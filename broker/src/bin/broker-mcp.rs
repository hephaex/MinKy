//! `broker-mcp` — the broker's stdio MCP server (official Rust SDK, `rmcp`).
//!
//! A **session-scoped** process (no resident daemon): it reads MCP requests
//! from stdin and writes responses to stdout, then exits naturally when the
//! client closes stdin (EOF). Every tool call loads [`BrokerConfig`] and opens
//! the state DB on demand — there is no shared long-lived connection.
//!
//! Tools:
//! - `load_working_set` — no args; returns the current working-set snapshot.
//! - `retrieve` — `{ query }`; runs the retrieval cascade.
//! - `record_access` — `{ rel_path, source }`; records a recall access.
//!
//! Adopted path: **rmcp** (official Rust MCP SDK) with the `#[tool]` macro
//! system and the built-in stdio transport. The transport-neutral handler
//! logic lives in [`broker::mcp_server`] so a manual JSON-RPC fallback could
//! reuse it verbatim if the SDK ever became unavailable.

use broker::config::{default_config_path, BrokerConfig};
use broker::mcp_server::{
    load_working_set_tool, now_epoch, open_storage, record_access_tool, retrieve_tool,
    CascadeOutcomeDto, RecordAccessResult, WorkingSetSnapshotDto,
};
use broker::minky_client::MinkyClient;

use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, Json, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::Deserialize;

/// Arguments for the `retrieve` tool.
#[derive(Debug, Deserialize, JsonSchema)]
struct RetrieveRequest {
    /// Natural-language query to run through the retrieval cascade.
    query: String,
}

/// Arguments for the `record_access` tool.
#[derive(Debug, Deserialize, JsonSchema)]
struct RecordAccessRequest {
    /// Memory path relative to the memory root (`/`-separated).
    rel_path: String,
    /// Access-source tag written to the audit log (e.g. `recall`, `edit`).
    source: String,
}

/// The MCP server. Holds only the loaded config; storage/minky are opened
/// per-call so the process stays stateless and daemon-free.
#[derive(Clone)]
struct BrokerMcp {
    config: BrokerConfig,
    tool_router: ToolRouter<Self>,
}

#[tool_router(router = tool_router)]
impl BrokerMcp {
    fn new(config: BrokerConfig) -> Self {
        Self {
            config,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "load_working_set",
        description = "Evaluate and return the current short-term working set (members + soft-demoted items)."
    )]
    async fn load_working_set(&self) -> Result<Json<WorkingSetSnapshotDto>, String> {
        let mut storage = open_storage(&self.config).map_err(|e| e.to_string())?;
        let snap = load_working_set_tool(&self.config, &mut storage, now_epoch())
            .map_err(|e| e.to_string())?;
        Ok(Json(snap))
    }

    #[tool(
        name = "retrieve",
        description = "Run the retrieval cascade for a query: working-set match first, then minky semantic search; returns a verdict plus evidence (sources only, never a synthesized answer)."
    )]
    async fn retrieve(
        &self,
        Parameters(req): Parameters<RetrieveRequest>,
    ) -> Result<Json<CascadeOutcomeDto>, String> {
        let mut storage = open_storage(&self.config).map_err(|e| e.to_string())?;
        let minky = MinkyClient::new(&self.config.minky).map_err(|e| e.to_string())?;
        let out = retrieve_tool(&self.config, &mut storage, &minky, &req.query, now_epoch())
            .await
            .map_err(|e| e.to_string())?;
        Ok(Json(out))
    }

    #[tool(
        name = "record_access",
        description = "Record a broker-mediated access (recall) for a memory path, re-entering it into the working set; returns the resulting membership/pin flags."
    )]
    async fn record_access(
        &self,
        Parameters(req): Parameters<RecordAccessRequest>,
    ) -> Result<Json<RecordAccessResult>, String> {
        let mut storage = open_storage(&self.config).map_err(|e| e.to_string())?;
        let res = record_access_tool(
            &self.config,
            &mut storage,
            &req.rel_path,
            &req.source,
            now_epoch(),
        )
        .map_err(|e| e.to_string())?;
        Ok(Json(res))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for BrokerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Local memory broker: short-term working set + long-term minky retrieval cascade. \
             Tools: load_working_set, retrieve, record_access.",
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Config is loaded once at startup (missing file -> defaults). Every tool
    // call re-opens the state DB, so there is no shared connection to leak.
    let config = BrokerConfig::load(&default_config_path())?;

    // Serve over stdio; `waiting()` returns when the client closes stdin (EOF),
    // giving natural session-scoped shutdown — no resident daemon.
    let service = BrokerMcp::new(config)
        .serve(rmcp::transport::stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}
