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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_engine_default_is_tesseract() {
        assert!(matches!(OcrEngine::default(), OcrEngine::Tesseract));
    }

    #[test]
    fn test_ocr_status_default_is_pending() {
        assert!(matches!(OcrStatus::default(), OcrStatus::Pending));
    }

    #[test]
    fn test_block_type_text_variant_exists() {
        let _ = BlockType::Text;
    }

    #[test]
    fn test_ocr_engine_all_variants_serialize() {
        let engines = [
            (OcrEngine::Tesseract, "\"tesseract\""),
            (OcrEngine::GoogleVision, "\"googlevision\""),
            (OcrEngine::AzureVision, "\"azurevision\""),
            (OcrEngine::AwsTextract, "\"awstextract\""),
        ];
        for (engine, expected) in engines {
            let json = serde_json::to_string(&engine).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn test_ocr_engine_roundtrip() {
        let engines = [
            OcrEngine::Tesseract,
            OcrEngine::GoogleVision,
            OcrEngine::AzureVision,
            OcrEngine::AwsTextract,
        ];
        for engine in engines {
            let json = serde_json::to_string(&engine).unwrap();
            let back: OcrEngine = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&back).unwrap();
            assert_eq!(json, json2);
        }
    }

    #[test]
    fn test_ocr_status_all_variants_serialize() {
        let statuses = [
            (OcrStatus::Pending, "\"pending\""),
            (OcrStatus::Processing, "\"processing\""),
            (OcrStatus::Completed, "\"completed\""),
            (OcrStatus::Failed, "\"failed\""),
        ];
        for (status, expected) in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn test_ocr_status_roundtrip() {
        let statuses = [
            OcrStatus::Pending,
            OcrStatus::Processing,
            OcrStatus::Completed,
            OcrStatus::Failed,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let back: OcrStatus = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&back).unwrap();
            assert_eq!(json, json2);
        }
    }

    #[test]
    fn test_block_type_all_variants_serialize() {
        let types = [
            (BlockType::Text, "\"text\""),
            (BlockType::Table, "\"table\""),
            (BlockType::Figure, "\"figure\""),
            (BlockType::Barcode, "\"barcode\""),
            (BlockType::Handwriting, "\"handwriting\""),
        ];
        for (bt, expected) in types {
            let json = serde_json::to_string(&bt).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn test_block_type_roundtrip() {
        let types = [
            BlockType::Text,
            BlockType::Table,
            BlockType::Figure,
            BlockType::Barcode,
            BlockType::Handwriting,
        ];
        for bt in types {
            let json = serde_json::to_string(&bt).unwrap();
            let back: BlockType = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&back).unwrap();
            assert_eq!(json, json2);
        }
    }

    #[test]
    fn test_bounding_box_serializes() {
        let bb = BoundingBox {
            x: 10,
            y: 20,
            width: 100,
            height: 50,
        };
        let json = serde_json::to_string(&bb).unwrap();
        assert!(json.contains("\"x\":10"));
        assert!(json.contains("\"y\":20"));
        assert!(json.contains("\"width\":100"));
        assert!(json.contains("\"height\":50"));
    }

    #[test]
    fn test_ocr_settings_serializes_and_deserializes() {
        let settings = OcrSettings {
            default_engine: OcrEngine::GoogleVision,
            default_languages: vec!["en".to_string(), "ko".to_string()],
            auto_ocr_on_upload: true,
            max_file_size_mb: 10,
            supported_formats: vec!["pdf".to_string(), "png".to_string()],
        };
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"googlevision\""));
        assert!(json.contains("\"auto_ocr_on_upload\":true"));
        assert!(json.contains("\"max_file_size_mb\":10"));

        let back: OcrSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.max_file_size_mb, 10);
        assert!(back.auto_ocr_on_upload);
    }

    #[test]
    fn test_ocr_request_all_none_by_default() {
        let req = OcrRequest {
            engine: None,
            languages: None,
            detect_orientation: None,
            enhance_image: None,
        };
        assert!(req.engine.is_none());
        assert!(req.languages.is_none());
        assert!(req.detect_orientation.is_none());
        assert!(req.enhance_image.is_none());
    }

    #[test]
    fn test_apply_ocr_request_append_defaults_to_none() {
        let req = ApplyOcrRequest {
            document_id: uuid::Uuid::new_v4(),
            append: None,
        };
        assert!(req.append.is_none());
    }

    #[test]
    fn test_apply_ocr_request_with_append_true() {
        let req = ApplyOcrRequest {
            document_id: uuid::Uuid::new_v4(),
            append: Some(true),
        };
        assert_eq!(req.append, Some(true));
    }
}
