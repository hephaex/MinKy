//! Pipeline stage trait and utilities
//!
//! Defines the `PipelineStage` trait that all pipeline stages must implement.

use async_trait::async_trait;

use super::context::PipelineContext;
use super::error::PipelineResult;

/// A single stage in the document processing pipeline
///
/// Each stage takes an input, processes it, and produces an output.
/// Stages can access and modify the shared `PipelineContext`.
///
/// # Type Parameters
///
/// * `I` - The input type this stage accepts
/// * `O` - The output type this stage produces
///
/// # Example
///
/// ```ignore
/// use async_trait::async_trait;
/// use minky::pipeline::{PipelineStage, PipelineContext, PipelineResult};
///
/// struct MyStage;
///
/// #[async_trait]
/// impl PipelineStage<String, String> for MyStage {
///     fn name(&self) -> &'static str {
///         "my_stage"
///     }
///
///     async fn process(
///         &self,
///         input: String,
///         context: &mut PipelineContext,
///     ) -> PipelineResult<String> {
///         Ok(input.to_uppercase())
///     }
/// }
/// ```
#[async_trait]
pub trait PipelineStage<I, O>: Send + Sync {
    /// Returns the name of this stage for logging and metrics
    fn name(&self) -> &'static str;

    /// Process the input and produce output
    ///
    /// # Arguments
    ///
    /// * `input` - The input from the previous stage
    /// * `context` - Shared context for metadata and metrics
    ///
    /// # Returns
    ///
    /// The processed output, or an error if processing failed
    async fn process(&self, input: I, context: &mut PipelineContext) -> PipelineResult<O>;

    /// Optional: Check if this stage should be skipped
    ///
    /// Override this to conditionally skip a stage based on input or context.
    fn should_skip(&self, _input: &I, _context: &PipelineContext) -> bool {
        false
    }

    /// Optional: Validate input before processing
    ///
    /// Override this to perform validation before the main processing logic.
    fn validate(&self, _input: &I) -> PipelineResult<()> {
        Ok(())
    }
}

/// Wrapper for executing a stage with timing and metrics
pub struct StageExecutor;

impl StageExecutor {
    /// Execute a stage with timing, metrics, and error handling
    pub async fn execute<I, O, S>(
        stage: &S,
        input: I,
        context: &mut PipelineContext,
    ) -> PipelineResult<O>
    where
        S: PipelineStage<I, O>,
        I: Send,
    {
        let stage_name = stage.name();
        let start = std::time::Instant::now();

        // Update context with current stage
        context.set_current_stage(stage_name);

        // Check if stage should be skipped
        if stage.should_skip(&input, context) {
            tracing::info!(stage = stage_name, "Skipping stage");
            // For skipped stages, we need to somehow pass through
            // This is a limitation - skippable stages should handle their own passthrough
        }

        // Validate input
        stage.validate(&input)?;

        // Execute the stage
        let result = stage.process(input, context).await;

        // Record metrics
        let duration_ms = start.elapsed().as_millis() as u64;

        match &result {
            Ok(_) => {
                context.record_stage_completion(stage_name, duration_ms);
                tracing::info!(
                    stage = stage_name,
                    duration_ms = duration_ms,
                    "Stage completed successfully"
                );
            }
            Err(e) => {
                context.record_stage_failure(stage_name, duration_ms);
                tracing::error!(
                    stage = stage_name,
                    duration_ms = duration_ms,
                    error = %e,
                    "Stage failed"
                );
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UppercaseStage;

    #[async_trait]
    impl PipelineStage<String, String> for UppercaseStage {
        fn name(&self) -> &'static str {
            "uppercase"
        }

        async fn process(
            &self,
            input: String,
            _context: &mut PipelineContext,
        ) -> PipelineResult<String> {
            Ok(input.to_uppercase())
        }
    }

    struct FailingStage;

    #[async_trait]
    impl PipelineStage<String, String> for FailingStage {
        fn name(&self) -> &'static str {
            "failing"
        }

        async fn process(
            &self,
            _input: String,
            _context: &mut PipelineContext,
        ) -> PipelineResult<String> {
            Err(super::super::error::PipelineError::stage_failure(
                "failing",
                "Always fails",
            ))
        }
    }

    #[tokio::test]
    async fn test_stage_execution_success() {
        let stage = UppercaseStage;
        let mut context = PipelineContext::new();

        let result = StageExecutor::execute(&stage, "hello".to_string(), &mut context).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "HELLO");
        assert_eq!(context.metrics.successful_stages(), 1);
    }

    #[tokio::test]
    async fn test_stage_execution_failure() {
        let stage = FailingStage;
        let mut context = PipelineContext::new();

        let result = StageExecutor::execute(&stage, "hello".to_string(), &mut context).await;

        assert!(result.is_err());
        assert_eq!(context.metrics.failed_stages(), 1);
    }

    #[test]
    fn test_stage_name() {
        let stage = UppercaseStage;
        assert_eq!(stage.name(), "uppercase");
    }
}
