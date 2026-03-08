//! Pipeline execution context
//!
//! The `PipelineContext` carries shared state across pipeline stages,
//! including the document being processed, accumulated metadata, and metrics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Context shared across all pipeline stages
///
/// This context is passed through each stage and accumulates metadata,
/// metrics, and intermediate results as the document flows through the pipeline.
#[derive(Debug, Clone)]
pub struct PipelineContext {
    /// Unique ID for this pipeline execution
    pub job_id: Uuid,

    /// Document ID (assigned after storage stage)
    pub document_id: Option<Uuid>,

    /// User ID who initiated the pipeline (for access control)
    pub user_id: Option<i32>,

    /// Pipeline execution start time
    pub started_at: DateTime<Utc>,

    /// Current stage being executed
    pub current_stage: Option<&'static str>,

    /// Arbitrary metadata accumulated during processing
    pub metadata: HashMap<String, serde_json::Value>,

    /// Execution metrics
    pub metrics: PipelineMetrics,

    /// Whether the pipeline should stop early
    pub cancelled: bool,

    /// Tags to apply to the document
    pub tags: Vec<String>,

    /// Category ID to assign
    pub category_id: Option<i32>,
}

impl PipelineContext {
    /// Create a new pipeline context
    pub fn new() -> Self {
        Self {
            job_id: Uuid::new_v4(),
            document_id: None,
            user_id: None,
            started_at: Utc::now(),
            current_stage: None,
            metadata: HashMap::new(),
            metrics: PipelineMetrics::default(),
            cancelled: false,
            tags: Vec::new(),
            category_id: None,
        }
    }

    /// Create a context with a specific user ID
    pub fn with_user(user_id: i32) -> Self {
        Self {
            user_id: Some(user_id),
            ..Self::new()
        }
    }

    /// Set the document ID
    pub fn set_document_id(&mut self, id: Uuid) {
        self.document_id = Some(id);
    }

    /// Set the current stage
    pub fn set_current_stage(&mut self, stage: &'static str) {
        self.current_stage = Some(stage);
    }

    /// Add metadata
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Serialize) {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.metadata.insert(key.into(), json_value);
        }
    }

    /// Get metadata value
    pub fn get_metadata<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.metadata
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Record that a stage completed successfully
    pub fn record_stage_completion(&mut self, stage: &'static str, duration_ms: u64) {
        self.metrics.stages_completed.push(StageMetric {
            name: stage.to_string(),
            duration_ms,
            success: true,
        });
    }

    /// Record that a stage failed
    pub fn record_stage_failure(&mut self, stage: &'static str, duration_ms: u64) {
        self.metrics.stages_completed.push(StageMetric {
            name: stage.to_string(),
            duration_ms,
            success: false,
        });
    }

    /// Get elapsed time since pipeline started
    pub fn elapsed_ms(&self) -> u64 {
        let elapsed = Utc::now() - self.started_at;
        elapsed.num_milliseconds().max(0) as u64
    }

    /// Check if the pipeline has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Cancel the pipeline
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.push(tag.into());
    }

    /// Set category
    pub fn set_category(&mut self, category_id: i32) {
        self.category_id = Some(category_id);
    }
}

impl Default for PipelineContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics collected during pipeline execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineMetrics {
    /// Metrics for each completed stage
    pub stages_completed: Vec<StageMetric>,

    /// Total tokens processed (for embedding)
    pub tokens_processed: u32,

    /// Number of chunks created
    pub chunks_created: u32,

    /// Number of embeddings generated
    pub embeddings_generated: u32,

    /// API calls made
    pub api_calls: u32,

    /// Total characters processed
    pub characters_processed: u64,
}

impl PipelineMetrics {
    /// Get total duration of all stages in milliseconds
    pub fn total_duration_ms(&self) -> u64 {
        self.stages_completed.iter().map(|s| s.duration_ms).sum()
    }

    /// Get number of successful stages
    pub fn successful_stages(&self) -> usize {
        self.stages_completed.iter().filter(|s| s.success).count()
    }

    /// Get number of failed stages
    pub fn failed_stages(&self) -> usize {
        self.stages_completed.iter().filter(|s| !s.success).count()
    }

    /// Add tokens processed
    pub fn add_tokens(&mut self, count: u32) {
        self.tokens_processed += count;
    }

    /// Add chunks created
    pub fn add_chunks(&mut self, count: u32) {
        self.chunks_created += count;
    }

    /// Add embeddings generated
    pub fn add_embeddings(&mut self, count: u32) {
        self.embeddings_generated += count;
    }

    /// Increment API calls
    pub fn increment_api_calls(&mut self) {
        self.api_calls += 1;
    }

    /// Add characters processed
    pub fn add_characters(&mut self, count: u64) {
        self.characters_processed += count;
    }
}

/// Metric for a single stage execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMetric {
    /// Stage name
    pub name: String,

    /// Duration in milliseconds
    pub duration_ms: u64,

    /// Whether the stage succeeded
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = PipelineContext::new();
        assert!(ctx.document_id.is_none());
        assert!(ctx.user_id.is_none());
        assert!(!ctx.cancelled);
    }

    #[test]
    fn test_context_with_user() {
        let ctx = PipelineContext::with_user(42);
        assert_eq!(ctx.user_id, Some(42));
    }

    #[test]
    fn test_metadata_operations() {
        let mut ctx = PipelineContext::new();
        ctx.set_metadata("key", "value");
        let result: Option<String> = ctx.get_metadata("key");
        assert_eq!(result, Some("value".to_string()));
    }

    #[test]
    fn test_stage_metrics() {
        let mut ctx = PipelineContext::new();
        ctx.record_stage_completion("parsing", 100);
        ctx.record_stage_completion("embedding", 500);
        ctx.record_stage_failure("analysis", 50);

        assert_eq!(ctx.metrics.successful_stages(), 2);
        assert_eq!(ctx.metrics.failed_stages(), 1);
        assert_eq!(ctx.metrics.total_duration_ms(), 650);
    }

    #[test]
    fn test_cancellation() {
        let mut ctx = PipelineContext::new();
        assert!(!ctx.is_cancelled());
        ctx.cancel();
        assert!(ctx.is_cancelled());
    }

    #[test]
    fn test_tags_and_category() {
        let mut ctx = PipelineContext::new();
        ctx.add_tag("rust");
        ctx.add_tag("backend");
        ctx.set_category(5);

        assert_eq!(ctx.tags, vec!["rust", "backend"]);
        assert_eq!(ctx.category_id, Some(5));
    }
}
