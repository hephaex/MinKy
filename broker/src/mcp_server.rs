//! MCP tool logic — the three broker tools as plain, testable functions plus
//! their serializable result DTOs.
//!
//! The transport layer (rmcp stdio server in `bin/broker-mcp.rs`) is a thin
//! shell over these functions: it loads [`BrokerConfig`], opens [`Storage`],
//! and dispatches tool calls here. Keeping the handler logic transport-neutral
//! means the same code backs the rmcp path and any fallback loop, and it can be
//! unit-tested without spawning a server.

use serde::Serialize;
use std::path::Path;

use crate::cascade::{self, CascadeError, CascadeOutcome, Evidence, Verdict};
use crate::config::{BrokerConfig, ConfigError};
use crate::minky_client::{MinkyClient, MinkyError};
use crate::storage::{Storage, StorageError};
use crate::working_set::{WorkingSet, WorkingSetError, WorkingSetItem, WorkingSetSnapshot};

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("working set error: {0}")]
    WorkingSet(#[from] WorkingSetError),
    #[error("cascade error: {0}")]
    Cascade(#[from] CascadeError),
    #[error("minky error: {0}")]
    Minky(#[from] MinkyError),
}

/// One evaluated memory item (serializable projection of [`WorkingSetItem`]).
#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
pub struct WorkingSetItemDto {
    pub rel_path: String,
    pub pinned: bool,
    pub effective_access: i64,
    pub in_working_set: bool,
}

impl From<&WorkingSetItem> for WorkingSetItemDto {
    fn from(item: &WorkingSetItem) -> Self {
        Self {
            rel_path: item.rel_path.clone(),
            pinned: item.pinned,
            effective_access: item.effective_access,
            in_working_set: item.in_working_set,
        }
    }
}

/// Serializable projection of [`WorkingSetSnapshot`].
#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
pub struct WorkingSetSnapshotDto {
    pub members: Vec<WorkingSetItemDto>,
    pub demoted: Vec<WorkingSetItemDto>,
}

impl From<&WorkingSetSnapshot> for WorkingSetSnapshotDto {
    fn from(snap: &WorkingSetSnapshot) -> Self {
        Self {
            members: snap.members.iter().map(WorkingSetItemDto::from).collect(),
            demoted: snap.demoted.iter().map(WorkingSetItemDto::from).collect(),
        }
    }
}

/// Serializable projection of [`Evidence`].
#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
pub struct EvidenceDto {
    pub title: String,
    pub snippet: String,
    pub source_id: String,
    pub similarity: f32,
}

impl From<&Evidence> for EvidenceDto {
    fn from(e: &Evidence) -> Self {
        Self {
            title: e.title.clone(),
            snippet: e.snippet.clone(),
            source_id: e.source_id.clone(),
            similarity: e.similarity,
        }
    }
}

/// Serializable projection of [`CascadeOutcome`]. `verdict` is stringified to
/// a stable lowercase-kebab tag so MCP clients get a plain enum string.
#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
pub struct CascadeOutcomeDto {
    pub verdict: String,
    pub reason: String,
    pub scores: Vec<f32>,
    pub hit_count: usize,
    pub evidence: Vec<EvidenceDto>,
    pub working_set_matches: Vec<String>,
    pub offline: bool,
}

/// Stable string tag for a [`Verdict`].
pub fn verdict_tag(v: &Verdict) -> &'static str {
    match v {
        Verdict::Sufficient => "sufficient",
        Verdict::Insufficient => "insufficient",
        Verdict::AskUser => "ask-user",
        Verdict::Offline => "offline",
    }
}

impl From<&CascadeOutcome> for CascadeOutcomeDto {
    fn from(o: &CascadeOutcome) -> Self {
        Self {
            verdict: verdict_tag(&o.verdict).to_string(),
            reason: o.reason.clone(),
            scores: o.scores.clone(),
            hit_count: o.hit_count,
            evidence: o.evidence.iter().map(EvidenceDto::from).collect(),
            working_set_matches: o.working_set_matches.clone(),
            offline: o.offline,
        }
    }
}

/// Result of a `record_access` tool call.
#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
pub struct RecordAccessResult {
    pub rel_path: String,
    pub source: String,
    /// Access was recorded (always true on success; explicit for MCP clients).
    pub recorded: bool,
    /// Whether the item is in the working set after recall (always true here).
    pub in_working_set: bool,
    /// Whether the item is a pinned always-include path.
    pub pinned: bool,
}

/// Current wall-clock time in epoch seconds.
pub fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Load config from `path`, falling back to defaults when the file is absent.
pub fn load_config(path: &Path) -> Result<BrokerConfig, ToolError> {
    Ok(BrokerConfig::load(path)?)
}

/// Open the broker state database named by `config.state_db.path`.
pub fn open_storage(config: &BrokerConfig) -> Result<Storage, ToolError> {
    Ok(Storage::open(&config.state_db.path)?)
}

/// `load_working_set` tool: evaluate and return the current working set.
pub fn load_working_set_tool(
    config: &BrokerConfig,
    storage: &mut Storage,
    now: i64,
) -> Result<WorkingSetSnapshotDto, ToolError> {
    let snapshot = {
        let mut ws = WorkingSet::new(&config.working_set, storage);
        ws.evaluate(now)?
    };
    Ok(WorkingSetSnapshotDto::from(&snapshot))
}

/// `retrieve` tool: run the retrieval cascade for `query`.
pub async fn retrieve_tool(
    config: &BrokerConfig,
    storage: &mut Storage,
    minky: &MinkyClient,
    query: &str,
    now: i64,
) -> Result<CascadeOutcomeDto, ToolError> {
    let outcome = cascade::run(query, config, storage, minky, now).await?;
    Ok(CascadeOutcomeDto::from(&outcome))
}

/// `record_access` tool: record a broker-mediated access (recall) for
/// `rel_path` tagged with `source`, re-entering it into the working set.
pub fn record_access_tool(
    config: &BrokerConfig,
    storage: &mut Storage,
    rel_path: &str,
    source: &str,
    now: i64,
) -> Result<RecordAccessResult, ToolError> {
    {
        let mut ws = WorkingSet::new(&config.working_set, storage);
        ws.record_access(rel_path, now, source)?;
    }
    // Read back the persisted flags so the client sees the resulting state.
    let item = storage.get_item(rel_path)?;
    let (in_working_set, pinned) = item
        .map(|i| (i.in_working_set, i.always_include))
        .unwrap_or((true, false));
    Ok(RecordAccessResult {
        rel_path: rel_path.to_string(),
        source: source.to_string(),
        recorded: true,
        in_working_set,
        pinned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BrokerConfig;
    use std::io::Write;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn now_ts() -> i64 {
        now_epoch()
    }

    struct Harness {
        _memory: TempDir,
        _keyfile: TempDir,
        config: BrokerConfig,
        storage: Storage,
    }

    fn harness(base_url: &str, files: &[(&str, &str)]) -> Harness {
        let memory = TempDir::new().unwrap();
        for (rel, content) in files {
            let full = memory.path().join(rel);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&full, content).unwrap();
        }
        let keydir = TempDir::new().unwrap();
        let key_path = keydir.path().join(".minky-key");
        let mut f = std::fs::File::create(&key_path).unwrap();
        writeln!(f, "test-key").unwrap();

        let mut config = BrokerConfig::default();
        config.working_set.memory_root = memory.path().to_path_buf();
        config.working_set.always_include = Vec::new();
        config.minky.base_url = base_url.to_string();
        config.minky.api_key_file = key_path;
        config.minky.timeout_secs = 2;

        let storage = Storage::open_in_memory().unwrap();
        Harness {
            _memory: memory,
            _keyfile: keydir,
            config,
            storage,
        }
    }

    fn client(config: &BrokerConfig) -> MinkyClient {
        MinkyClient::new(&config.minky).unwrap()
    }

    fn envelope(results: &[(f32, &str)]) -> serde_json::Value {
        let arr: Vec<_> = results
            .iter()
            .enumerate()
            .map(|(i, (sim, title))| {
                serde_json::json!({
                    "document_id": format!("00000000-0000-0000-0000-{:012}", i),
                    "chunk_id": null,
                    "chunk_text": format!("snippet for {title}"),
                    "similarity": sim,
                    "document_title": title,
                })
            })
            .collect();
        serde_json::json!({
            "success": true,
            "data": { "results": arr, "total": arr.len(), "query": "q" }
        })
    }

    async fn mount_semantic(server: &MockServer, results: &[(f32, &str)]) {
        Mock::given(method("POST"))
            .and(path("/api/search/semantic"))
            .respond_with(ResponseTemplate::new(200).set_body_json(envelope(results)))
            .mount(server)
            .await;
    }

    #[test]
    fn load_working_set_reports_pins_and_recency() {
        let mut h = harness("http://unused", &[("IDENTITY.md", "id"), ("notes.md", "n")]);
        h.config.working_set.always_include = vec!["IDENTITY.md".to_string()];

        let snap = load_working_set_tool(&h.config, &mut h.storage, now_ts()).unwrap();

        // Both files are fresh -> both members; the pin sorts first.
        let paths: Vec<&str> = snap.members.iter().map(|m| m.rel_path.as_str()).collect();
        assert!(paths.contains(&"IDENTITY.md"));
        assert!(paths.contains(&"notes.md"));
        let pin = snap
            .members
            .iter()
            .find(|m| m.rel_path == "IDENTITY.md")
            .unwrap();
        assert!(pin.pinned);
        assert!(pin.in_working_set);
    }

    #[test]
    fn load_working_set_demotes_stale_non_pins() {
        let mut h = harness("http://unused", &[("old.md", "stale")]);
        // Backdate the file well beyond the 30-day window.
        let old = now_ts() - 400 * 86_400;
        let ft = filetime::FileTime::from_unix_time(old, 0);
        filetime::set_file_mtime(h.config.working_set.memory_root.join("old.md"), ft).unwrap();

        let snap = load_working_set_tool(&h.config, &mut h.storage, now_ts()).unwrap();
        assert!(snap.members.is_empty());
        assert_eq!(snap.demoted.len(), 1);
        assert_eq!(snap.demoted[0].rel_path, "old.md");
        assert!(!snap.demoted[0].in_working_set);
    }

    #[test]
    fn record_access_recalls_and_reports_flags() {
        let mut h = harness("http://unused", &[("doc.md", "content")]);

        let res =
            record_access_tool(&h.config, &mut h.storage, "doc.md", "recall", now_ts()).unwrap();
        assert!(res.recorded);
        assert!(res.in_working_set);
        assert!(!res.pinned);
        assert_eq!(res.rel_path, "doc.md");
        assert_eq!(res.source, "recall");

        // The access clock was persisted.
        let item = h.storage.get_item("doc.md").unwrap().unwrap();
        assert!(item.last_accessed.is_some());
        assert!(item.in_working_set);
    }

    #[test]
    fn record_access_reports_pinned_flag() {
        let mut h = harness("http://unused", &[("IDENTITY.md", "id")]);
        h.config.working_set.always_include = vec!["IDENTITY.md".to_string()];

        let res =
            record_access_tool(&h.config, &mut h.storage, "IDENTITY.md", "recall", now_ts()).unwrap();
        assert!(res.pinned);
        assert!(res.in_working_set);
    }

    #[tokio::test]
    async fn retrieve_working_set_hit_short_circuits() {
        // A working-set file whose content matches the query -> Sufficient with
        // no minky call (no mock mounted, so any HTTP call would fail).
        let server = MockServer::start().await;
        let mut h = harness(&server.uri(), &[("kubernetes.md", "kubernetes deploy notes")]);

        let out = retrieve_tool(
            &h.config,
            &mut h.storage,
            &client(&h.config),
            "kubernetes",
            now_ts(),
        )
        .await
        .unwrap();

        assert_eq!(out.verdict, "sufficient");
        assert!(out.working_set_matches.contains(&"kubernetes.md".to_string()));
    }

    #[tokio::test]
    async fn retrieve_falls_through_to_minky() {
        let server = MockServer::start().await;
        mount_semantic(&server, &[(0.8, "doc-a"), (0.7, "doc-b")]).await;
        let mut h = harness(&server.uri(), &[]);

        let out = retrieve_tool(
            &h.config,
            &mut h.storage,
            &client(&h.config),
            "unmatched query",
            now_ts(),
        )
        .await
        .unwrap();

        assert_eq!(out.verdict, "sufficient");
        assert_eq!(out.hit_count, 2);
        assert_eq!(out.scores, vec![0.8, 0.7]);
        assert_eq!(out.evidence.len(), 2);
        assert!(!out.offline);
    }

    #[test]
    fn verdict_tags_are_stable() {
        assert_eq!(verdict_tag(&Verdict::Sufficient), "sufficient");
        assert_eq!(verdict_tag(&Verdict::Insufficient), "insufficient");
        assert_eq!(verdict_tag(&Verdict::AskUser), "ask-user");
        assert_eq!(verdict_tag(&Verdict::Offline), "offline");
    }
}
