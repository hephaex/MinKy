//! `broker.toml` configuration loader.
//!
//! Default location: `~/.config/gyeol/broker.toml`. Every threshold is
//! externalized (tunable without recompiling) per the approved plan decision
//! (a): initial values `min_score=0.55`, `sufficient_top1=0.65`,
//! `min_results=2`, `top_k=8`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse broker.toml: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid config: {0}")]
    Invalid(String),
    #[error("wiki output_dir {0} resolves inside memory_root or the sync-shared config tree; refused")]
    UnsafeOutputDir(PathBuf),
}

/// Verdict thresholds for the retrieval cascade (approved plan decision (a)).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct VerdictConfig {
    /// Below this top-1 similarity after the minky pass, verdict = ask-user.
    pub min_score: f64,
    /// At or above this top-1 similarity (with `min_results` hits), verdict = sufficient.
    pub sufficient_top1: f64,
    /// Minimum number of hits for a sufficient verdict.
    pub min_results: u32,
    /// Number of results requested from minky `/search/semantic`.
    pub top_k: u32,
}

impl Default for VerdictConfig {
    fn default() -> Self {
        Self {
            min_score: 0.55,
            sufficient_top1: 0.65,
            min_results: 2,
            top_k: 8,
        }
    }
}

/// Long-term memory backend (minky-rust API on `ct-minky`, CAS internal
/// network `172.20.0.0/14`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct MinkyConfig {
    /// Base URL, e.g. `http://172.20.0.110:8000` (ct-minky on the CAS
    /// Proxmox internal network). The previously used `192.168.29.22:8001`
    /// is unreachable from the CAS network (`172.20.0.0/14`) and from a
    /// developer machine outside it — confirmed by direct route/connect
    /// testing (2026-07-07 redteam). Callers outside the CAS network must
    /// tunnel/VPN in and override this value; it is never hardcoded to an
    /// address only reachable from one specific host.
    pub base_url: String,
    /// File containing the long-lived M2M API key (never inline in TOML).
    pub api_key_file: PathBuf,
    /// Connect/read timeout in seconds before the offline degradation path.
    pub timeout_secs: u64,
}

impl Default for MinkyConfig {
    fn default() -> Self {
        Self {
            base_url: "http://172.20.0.110:8000".to_string(),
            api_key_file: default_config_dir().join(".minky-broker-key"),
            timeout_secs: 5,
        }
    }
}

/// Working-set policy (approved spec: always-include 3 files + 1-month access window).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct WorkingSetConfig {
    /// Root of the gyeol memory tree.
    pub memory_root: PathBuf,
    /// Access window in days; items untouched longer leave the working set (soft).
    pub window_days: u32,
    /// Paths (relative to `memory_root`) that are pinned and never demoted.
    pub always_include: Vec<String>,
}

impl Default for WorkingSetConfig {
    fn default() -> Self {
        Self {
            memory_root: home_dir().join("Simon/Gaeyang/.memory"),
            window_days: 30,
            always_include: vec![
                "IDENTITY.md".to_string(),
                "SELF.md".to_string(),
                "episodes/_recent.md".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default, deny_unknown_fields)]
pub struct BrokerConfig {
    pub verdict: VerdictConfig,
    pub minky: MinkyConfig,
    pub working_set: WorkingSetConfig,
    /// Location of the broker state database.
    pub state_db: StateDbConfig,
    /// Synthesis endpoint for on-demand wiki generation (`generate_wiki`).
    pub synthesis: SynthesisConfig,
    /// Wiki generation output policy (`generate_wiki`).
    pub wiki: WikiConfig,
}

/// Synthesis endpoint for on-demand wiki generation (`generate_wiki`).
///
/// Disabled by default (`enable = false`) to preserve the broker's default
/// no-synthesis, observation-only posture. This is an explicit, scoped
/// exception to the system-wide vendor-neutral principle: when enabled, the
/// broker calls a user-configured OpenAI-compatible endpoint directly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct SynthesisConfig {
    pub enable: bool,
    pub base_url: String,
    /// File containing the API key (never inline in TOML). May be absent for
    /// unauthenticated local endpoints (e.g. Ollama).
    pub api_key_file: PathBuf,
    pub model: String,
    pub timeout_secs: u64,
}

impl Default for SynthesisConfig {
    fn default() -> Self {
        Self {
            enable: false,
            base_url: "http://localhost:11434/v1".to_string(),
            api_key_file: default_config_dir().join(".synth-key"),
            model: "gpt-4o-mini".to_string(),
            timeout_secs: 60,
        }
    }
}

/// Wiki generation output policy (`generate_wiki`).
///
/// `output_dir` MUST resolve outside `working_set.memory_root` and outside
/// the sync-shared config tree, or generated wiki pages could be re-ingested
/// as their own evidence (self-reference feedback loop). Default lives under
/// `~/.local/state`, deliberately outside `~/.config/gyeol` (shared with
/// `sync-memory-to-minky.sh`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct WikiConfig {
    pub output_dir: PathBuf,
    pub max_topic_slug_len: usize,
}

impl Default for WikiConfig {
    fn default() -> Self {
        Self {
            output_dir: home_dir().join(".local/state/gyeol-wiki"),
            max_topic_slug_len: 64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct StateDbConfig {
    pub path: PathBuf,
}

impl Default for StateDbConfig {
    fn default() -> Self {
        Self {
            path: default_config_dir().join("broker.sqlite"),
        }
    }
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// `~/.config/gyeol` — shared with the existing sync tooling.
pub fn default_config_dir() -> PathBuf {
    home_dir().join(".config/gyeol")
}

/// Default config file path: `~/.config/gyeol/broker.toml`.
pub fn default_config_path() -> PathBuf {
    default_config_dir().join("broker.toml")
}

impl BrokerConfig {
    /// Parse from a TOML string. Missing sections/fields fall back to defaults;
    /// unknown fields are rejected to surface typos early.
    pub fn from_toml_str(raw: &str) -> Result<Self, ConfigError> {
        let cfg: BrokerConfig = toml::from_str(raw)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Load from a file path. A missing file yields the full default config
    /// (first-run experience); any other IO error is surfaced.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(raw) => Self::from_toml_str(&raw),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let cfg = Self::default();
                cfg.validate()?;
                Ok(cfg)
            }
            Err(e) => Err(ConfigError::Io {
                path: path.to_path_buf(),
                source: e,
            }),
        }
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let v = &self.verdict;
        for (name, value) in [("min_score", v.min_score), ("sufficient_top1", v.sufficient_top1)] {
            if !(0.0..=1.0).contains(&value) {
                return Err(ConfigError::Invalid(format!(
                    "verdict.{name} must be within [0.0, 1.0], got {value}"
                )));
            }
        }
        if v.sufficient_top1 < v.min_score {
            return Err(ConfigError::Invalid(format!(
                "verdict.sufficient_top1 ({}) must be >= verdict.min_score ({})",
                v.sufficient_top1, v.min_score
            )));
        }
        if v.min_results == 0 {
            return Err(ConfigError::Invalid(
                "verdict.min_results must be >= 1".to_string(),
            ));
        }
        if v.top_k < v.min_results {
            return Err(ConfigError::Invalid(format!(
                "verdict.top_k ({}) must be >= verdict.min_results ({})",
                v.top_k, v.min_results
            )));
        }
        if self.working_set.window_days == 0 {
            return Err(ConfigError::Invalid(
                "working_set.window_days must be >= 1".to_string(),
            ));
        }
        if self.minky.base_url.is_empty() {
            return Err(ConfigError::Invalid(
                "minky.base_url must not be empty".to_string(),
            ));
        }
        if self.synthesis.enable && self.synthesis.base_url.is_empty() {
            return Err(ConfigError::Invalid(
                "synthesis.base_url must not be empty when synthesis.enable is true".to_string(),
            ));
        }
        if self.synthesis.timeout_secs == 0 {
            return Err(ConfigError::Invalid(
                "synthesis.timeout_secs must be >= 1".to_string(),
            ));
        }
        if self.wiki.max_topic_slug_len == 0 {
            return Err(ConfigError::Invalid(
                "wiki.max_topic_slug_len must be >= 1".to_string(),
            ));
        }
        Ok(())
    }
}
/// Self-reference guard for `generate_wiki` output (spec Fact#9 / ADR-01).
///
/// Deliberately separate from [`BrokerConfig::validate`], which MUST stay
/// pure and I/O-free: this function performs filesystem `canonicalize`
/// calls, so injecting it into `validate()` would make first-run/default
/// config loading depend on filesystem state and could regress the existing
/// pure config unit tests.
///
/// Guard semantics (approved plan, N2 revision):
/// - **fail-open**: if `output_dir` cannot be canonicalized (e.g. it does not
///   exist yet), the path is treated as known-safe and accepted. A
///   not-yet-existing path cannot already be a symlink swapped into
///   `memory_root`/the sync-shared tree; the write-time guard in `wiki.rs`
///   and the scan-time guard in `working_set.rs` provide the real defense in
///   depth, so fail-open here is safe.
/// - **ancestor fallback**: if `output_dir` itself does not exist, walk up to
///   the nearest existing ancestor and canonicalize that instead, so a
///   symlinked *parent* directory is still caught before the first write. If
///   no ancestor exists either, fail-open.
/// - Both `memory_root` and `config_dir` are canonicalized for comparison;
///   if either fails to canonicalize, that boundary is skipped (fail-open
///   for that boundary only).
pub fn resolve_safe_output_dir(
    output_dir: &Path,
    memory_root: &Path,
    config_dir: &Path,
) -> Result<PathBuf, ConfigError> {
    let canonical_or_ancestor = |path: &Path| -> Option<PathBuf> {
        if let Ok(canonical) = path.canonicalize() {
            return Some(canonical);
        }
        let mut ancestor = path.parent();
        while let Some(dir) = ancestor {
            if let Ok(canonical) = dir.canonicalize() {
                return Some(canonical);
            }
            ancestor = dir.parent();
        }
        None
    };

    let Some(canonical_output) = canonical_or_ancestor(output_dir) else {
        // Neither output_dir nor any ancestor exists yet: fail-open.
        return Ok(output_dir.to_path_buf());
    };

    for boundary in [memory_root, config_dir] {
        if let Ok(canonical_boundary) = boundary.canonicalize() {
            if canonical_output.starts_with(&canonical_boundary) {
                return Err(ConfigError::UnsafeOutputDir(output_dir.to_path_buf()));
            }
        }
        // Boundary does not exist / cannot be canonicalized: fail-open for
        // that boundary (nothing to compare against yet).
    }

    Ok(output_dir.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_approved_plan_decision_a() {
        let cfg = BrokerConfig::default();
        assert_eq!(cfg.verdict.min_score, 0.55);
        assert_eq!(cfg.verdict.sufficient_top1, 0.65);
        assert_eq!(cfg.verdict.min_results, 2);
        assert_eq!(cfg.verdict.top_k, 8);
        assert_eq!(cfg.working_set.window_days, 30);
        assert_eq!(
            cfg.working_set.always_include,
            vec!["IDENTITY.md", "SELF.md", "episodes/_recent.md"]
        );
    }

    #[test]
    fn empty_toml_yields_defaults() {
        let cfg = BrokerConfig::from_toml_str("").unwrap();
        assert_eq!(cfg, BrokerConfig::default());
    }

    #[test]
    fn partial_toml_overrides_only_named_fields() {
        let cfg = BrokerConfig::from_toml_str(
            r#"
[verdict]
min_score = 0.4

[minky]
base_url = "http://localhost:8000"
"#,
        )
        .unwrap();
        assert_eq!(cfg.verdict.min_score, 0.4);
        assert_eq!(cfg.verdict.top_k, 8, "unnamed fields keep defaults");
        assert_eq!(cfg.minky.base_url, "http://localhost:8000");
        assert_eq!(cfg.minky.timeout_secs, 5);
    }

    #[test]
    fn unknown_field_is_rejected() {
        let err = BrokerConfig::from_toml_str("[verdict]\nmin_scoer = 0.4\n").unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)));
    }

    #[test]
    fn invalid_threshold_ordering_is_rejected() {
        let err = BrokerConfig::from_toml_str(
            "[verdict]\nmin_score = 0.9\nsufficient_top1 = 0.6\n",
        )
        .unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)));
    }

    #[test]
    fn out_of_range_score_is_rejected() {
        let err = BrokerConfig::from_toml_str("[verdict]\nmin_score = 1.5\n").unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)));
    }

    #[test]
    fn zero_window_days_is_rejected() {
        let err = BrokerConfig::from_toml_str("[working_set]\nwindow_days = 0\n").unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)));
    }

    #[test]
    fn missing_file_loads_defaults() {
        let cfg = BrokerConfig::load(Path::new("/nonexistent/broker.toml")).unwrap();
        assert_eq!(cfg, BrokerConfig::default());
    }

    #[test]
    fn load_reads_file_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broker.toml");
        std::fs::write(&path, "[verdict]\ntop_k = 12\n").unwrap();
        let cfg = BrokerConfig::load(&path).unwrap();
        assert_eq!(cfg.verdict.top_k, 12);
    }

    #[test]
    fn synthesis_disabled_by_default() {
        let cfg = BrokerConfig::default();
        assert!(!cfg.synthesis.enable);
        assert_eq!(cfg.synthesis.base_url, "http://localhost:11434/v1");
        assert_eq!(cfg.synthesis.model, "gpt-4o-mini");
        assert_eq!(cfg.synthesis.timeout_secs, 60);
    }

    #[test]
    fn wiki_output_dir_defaults_outside_sync_shared_config_dir() {
        let cfg = BrokerConfig::default();
        assert_eq!(cfg.wiki.max_topic_slug_len, 64);
        assert!(
            !cfg.wiki.output_dir.starts_with(default_config_dir()),
            "wiki.output_dir must default outside the sync-shared config dir (self-reference guard)"
        );
    }

    #[test]
    fn synthesis_enabled_with_empty_base_url_is_rejected() {
        let err = BrokerConfig::from_toml_str("[synthesis]\nenable = true\nbase_url = \"\"\n")
            .unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)));
    }

    #[test]
    fn zero_synthesis_timeout_is_rejected() {
        let err =
            BrokerConfig::from_toml_str("[synthesis]\ntimeout_secs = 0\n").unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)));
    }

    #[test]
    fn zero_max_topic_slug_len_is_rejected() {
        let err = BrokerConfig::from_toml_str("[wiki]\nmax_topic_slug_len = 0\n").unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)));
    }

    // -- resolve_safe_output_dir (G002, self-reference guard) --

    #[test]
    fn output_dir_under_memory_root_is_rejected() {
        let base = tempfile::tempdir().unwrap();
        let memory_root = base.path().join("memory");
        let config_dir = base.path().join("config");
        std::fs::create_dir_all(&memory_root).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();
        let output_dir = memory_root.join("wiki");
        std::fs::create_dir_all(&output_dir).unwrap();

        let err = resolve_safe_output_dir(&output_dir, &memory_root, &config_dir).unwrap_err();
        assert!(matches!(err, ConfigError::UnsafeOutputDir(_)));
    }

    #[test]
    fn output_dir_under_sync_shared_config_dir_is_rejected() {
        let base = tempfile::tempdir().unwrap();
        let memory_root = base.path().join("memory");
        let config_dir = base.path().join("config");
        std::fs::create_dir_all(&memory_root).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();
        let output_dir = config_dir.join("wiki");
        std::fs::create_dir_all(&output_dir).unwrap();

        let err = resolve_safe_output_dir(&output_dir, &memory_root, &config_dir).unwrap_err();
        assert!(matches!(err, ConfigError::UnsafeOutputDir(_)));
    }

    #[test]
    fn nonexistent_output_dir_fails_open() {
        let base = tempfile::tempdir().unwrap();
        let memory_root = base.path().join("memory");
        let config_dir = base.path().join("config");
        std::fs::create_dir_all(&memory_root).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();
        // output_dir and every ancestor up to base are absent.
        let output_dir = base.path().join("does/not/exist/wiki");

        let resolved = resolve_safe_output_dir(&output_dir, &memory_root, &config_dir).unwrap();
        assert_eq!(resolved, output_dir);
    }

    #[test]
    fn ancestor_fallback_catches_symlinked_parent() {
        let base = tempfile::tempdir().unwrap();
        let memory_root = base.path().join("memory");
        std::fs::create_dir_all(&memory_root).unwrap();
        let config_dir = base.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();

        // `linked` is a symlink into memory_root; output_dir itself does not
        // exist yet, but its parent (the symlink) does.
        let linked = base.path().join("linked");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&memory_root, &linked).unwrap();
        let output_dir = linked.join("wiki");

        #[cfg(unix)]
        {
            let err =
                resolve_safe_output_dir(&output_dir, &memory_root, &config_dir).unwrap_err();
            assert!(matches!(err, ConfigError::UnsafeOutputDir(_)));
        }
    }

    #[test]
    fn safe_output_dir_outside_both_boundaries_is_accepted() {
        let base = tempfile::tempdir().unwrap();
        let memory_root = base.path().join("memory");
        let config_dir = base.path().join("config");
        let output_dir = base.path().join("state/gyeol-wiki");
        std::fs::create_dir_all(&memory_root).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&output_dir).unwrap();

        let resolved = resolve_safe_output_dir(&output_dir, &memory_root, &config_dir).unwrap();
        assert_eq!(resolved, output_dir);
    }
}
