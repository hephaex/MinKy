//! Document Processing Pipeline
//!
//! A modular, composable pipeline for processing documents through multiple stages:
//! ingestion, parsing, metadata extraction, chunking, embedding, AI analysis, and storage.
//!
//! # Architecture
//!
//! The pipeline follows a stage-based architecture inspired by Quarkdown, where each
//! stage implements the `PipelineStage` trait and can be composed together using
//! the `DocumentPipelineBuilder`.
//!
//! ```text
//! ┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐
//! │  Ingestion   │───▶│   Parsing    │───▶│  MetadataExtraction  │
//! │    Stage     │    │    Stage     │    │       Stage          │
//! └──────────────┘    └──────────────┘    └──────────────────────┘
//!                                                   │
//!                                                   ▼
//! ┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐
//! │   Storage    │◀───│  Embedding   │◀───│    Chunking          │
//! │    Stage     │    │    Stage     │    │       Stage          │
//! └──────────────┘    └──────────────┘    └──────────────────────┘
//!        │
//!        ▼
//! ┌──────────────┐    ┌──────────────┐
//! │   Indexing   │───▶│ AIAnalysis   │───▶ Complete
//! │    Stage     │    │    Stage     │
//! └──────────────┘    └──────────────┘
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use minky::pipeline::{DocumentPipeline, IngestionInput};
//!
//! async fn process_document(pipeline: &DocumentPipeline, content: &str) {
//!     let input = IngestionInput::from_text("My Document", content);
//!     let result = pipeline.process(input).await;
//! }
//! ```

pub mod builder;
pub mod context;
pub mod error;
pub mod stage;
pub mod stages;

pub use builder::{DocumentPipeline, DocumentPipelineBuilder};
pub use context::{PipelineContext, PipelineMetrics};
pub use error::{PipelineError, PipelineResult};
pub use stage::PipelineStage;
pub use stages::*;
