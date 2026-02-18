use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// OCR engine type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OcrEngine {
    #[default]
    Tesseract,
    GoogleVision,
    AzureVision,
    AwsTextract,
}

/// OCR job status
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OcrStatus {
    #[default]
    Pending,
    Processing,
    Completed,
    Failed,
}

/// OCR request
#[derive(Debug, Deserialize)]
pub struct OcrRequest {
    pub engine: Option<OcrEngine>,
    pub languages: Option<Vec<String>>,
    pub detect_orientation: Option<bool>,
    pub enhance_image: Option<bool>,
}

/// OCR job
#[derive(Debug, Serialize)]
pub struct OcrJob {
    pub id: String,
    pub attachment_id: uuid::Uuid,
    pub user_id: i32,
    pub engine: OcrEngine,
    pub status: OcrStatus,
    pub progress_percent: i32,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// OCR result
#[derive(Debug, Serialize)]
pub struct OcrResult {
    pub job_id: String,
    pub text: String,
    pub confidence: f32,
    pub pages: Vec<OcrPage>,
    pub metadata: OcrMetadata,
    pub processing_time_ms: i64,
}

/// OCR page result
#[derive(Debug, Clone, Serialize)]
pub struct OcrPage {
    pub page_number: i32,
    pub text: String,
    pub confidence: f32,
    pub blocks: Vec<TextBlock>,
    pub width: i32,
    pub height: i32,
}

/// Text block in OCR result
#[derive(Debug, Clone, Serialize)]
pub struct TextBlock {
    pub text: String,
    pub confidence: f32,
    pub bounding_box: BoundingBox,
    pub block_type: BlockType,
}

/// Bounding box for text region
#[derive(Debug, Clone, Serialize)]
pub struct BoundingBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Block type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockType {
    Text,
    Table,
    Figure,
    Barcode,
    Handwriting,
}

/// OCR metadata
#[derive(Debug, Clone, Serialize)]
pub struct OcrMetadata {
    pub engine: OcrEngine,
    pub languages_detected: Vec<String>,
    pub orientation: i32,
    pub total_pages: i32,
    pub word_count: i32,
}

/// Apply OCR text to document request
#[derive(Debug, Deserialize)]
pub struct ApplyOcrRequest {
    pub document_id: uuid::Uuid,
    pub append: Option<bool>,
}

/// OCR settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrSettings {
    pub default_engine: OcrEngine,
    pub default_languages: Vec<String>,
    pub auto_ocr_on_upload: bool,
    pub max_file_size_mb: i32,
    pub supported_formats: Vec<String>,
}
