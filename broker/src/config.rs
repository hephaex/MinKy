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

/// Long-term memory backend (minky on CT306 or a local instance).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct MinkyConfig {
    /// Base URL, e.g. `http://192.168.29.22:8001`.
    pub base_url: String,
    /// File containing the long-lived M2M API key (never inline in TOML).
    pub api_key_file: PathBuf,
    /// Connect/read timeout in seconds before the offline degradation path.
    pub timeout_secs: u64,
}

impl Default for MinkyConfig {
    fn default() -> Self {
        Self {
            base_url: "http://192.168.29.22:8001".to_string(),
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
        Ok(())
    }
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
}
