//! Local memory broker library.
//!
//! Layered per the approved plan (WP0):
//! - [`config`]: `broker.toml` loader with verdict thresholds, minky endpoint,
//!   and working-set policy. All natural-language-facing defaults live here.
//! - [`storage`]: `broker.sqlite` state — the access clock and working-set
//!   membership. The broker never deletes memory files; demotion is soft
//!   (working-set exclusion only), evaluated lazily at query time.

pub mod cascade;
pub mod config;
pub mod mcp_server;
pub mod minky_client;
pub mod storage;
pub mod synth_client;
pub mod wiki;
pub mod working_set;

pub use cascade::{run, CascadeError, CascadeOutcome, Evidence, Verdict};
pub use config::BrokerConfig;
pub use minky_client::{MinkyClient, MinkyError, SemanticHit};
pub use storage::Storage;
pub use working_set::{WorkingSet, WorkingSetItem, WorkingSetSnapshot};
