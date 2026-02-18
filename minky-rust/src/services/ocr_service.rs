use anyhow::Result;
use chrono::Utc;
use std::process::Command;

use crate::models::{
    ApplyOcrRequest, BlockType, BoundingBox, OcrEngine, OcrJob, OcrMetadata, OcrPage, OcrRequest,
    OcrResult, OcrSettings, OcrStatus, TextBlock,
};

/// OCR service for document text extraction
pub struct OcrService {
    settings: OcrSettings,
}

impl OcrService {
    pub fn new() -> Self {
        Self {
            settings: OcrSettings {
                default_engine: OcrEngine::Tesseract,
                default_languages: vec!["eng".to_string(), "kor".to_string()],
                auto_ocr_on_upload: false,
                max_file_size_mb: 50,
                supported_formats: vec![
                    "pdf".to_string(),
                    "png".to_string(),
                    "jpg".to_string(),
                    "jpeg".to_string(),
                    "tiff".to_string(),
                    "bmp".to_string(),
                ],
            },
        }
    }

    /// Start OCR job
    pub async fn start_ocr(
        &self,
        attachment_id: uuid::Uuid,
        user_id: i32,
        request: OcrRequest,
    ) -> Result<OcrJob> {
        let job_id = uuid::Uuid::new_v4().to_string();
        let engine = request.engine.unwrap_or(self.settings.default_engine.clone());

        Ok(OcrJob {
            id: job_id,
            attachment_id,
            user_id,
            engine,
            status: OcrStatus::Pending,
            progress_percent: 0,
            created_at: Utc::now(),
            completed_at: None,
        })
    }

    /// Process image with Tesseract OCR
    pub fn process_with_tesseract(&self, image_path: &str, languages: &[String]) -> Result<OcrResult> {
        let start_time = std::time::Instant::now();
        let lang_param = languages.join("+");

        let output = Command::new("tesseract")
            .arg(image_path)
            .arg("stdout")
            .arg("-l")
            .arg(&lang_param)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Tesseract failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let text = String::from_utf8_lossy(&output.stdout).to_string();
        let word_count = text.split_whitespace().count() as i32;

        let page = OcrPage {
            page_number: 1,
            text: text.clone(),
            confidence: 0.85,
            blocks: vec![TextBlock {
                text: text.clone(),
                confidence: 0.85,
                bounding_box: BoundingBox {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                },
                block_type: BlockType::Text,
            }],
            width: 0,
            height: 0,
        };

        Ok(OcrResult {
            job_id: uuid::Uuid::new_v4().to_string(),
            text,
            confidence: 0.85,
            pages: vec![page],
            metadata: OcrMetadata {
                engine: OcrEngine::Tesseract,
                languages_detected: languages.to_vec(),
                orientation: 0,
                total_pages: 1,
                word_count,
            },
            processing_time_ms: start_time.elapsed().as_millis() as i64,
        })
    }

    /// Process PDF with OCR
    pub fn process_pdf(&self, pdf_path: &str, languages: &[String]) -> Result<OcrResult> {
        let start_time = std::time::Instant::now();

        // Convert PDF to images and process each page
        // This is a simplified implementation
        let output = Command::new("pdftotext")
            .arg(pdf_path)
            .arg("-")
            .output()?;

        let text = if output.status.success() {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            // Fall back to OCR
            self.process_with_tesseract(pdf_path, languages)?
                .text
        };

        let word_count = text.split_whitespace().count() as i32;

        Ok(OcrResult {
            job_id: uuid::Uuid::new_v4().to_string(),
            text: text.clone(),
            confidence: 0.90,
            pages: vec![OcrPage {
                page_number: 1,
                text,
                confidence: 0.90,
                blocks: vec![],
                width: 0,
                height: 0,
            }],
            metadata: OcrMetadata {
                engine: OcrEngine::Tesseract,
                languages_detected: languages.to_vec(),
                orientation: 0,
                total_pages: 1,
                word_count,
            },
            processing_time_ms: start_time.elapsed().as_millis() as i64,
        })
    }

    /// Get OCR job status
    pub async fn get_job_status(&self, _job_id: &str) -> Result<Option<OcrJob>> {
        // TODO: Implement job queue
        Ok(None)
    }

    /// Get OCR settings
    pub fn get_settings(&self) -> &OcrSettings {
        &self.settings
    }

    /// Update OCR settings
    pub fn update_settings(&mut self, settings: OcrSettings) {
        self.settings = settings;
    }

    /// Check if file format is supported
    pub fn is_supported_format(&self, filename: &str) -> bool {
        let extension = filename
            .rsplit('.')
            .next()
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        self.settings.supported_formats.contains(&extension)
    }

    /// Estimate OCR processing time
    pub fn estimate_processing_time(&self, file_size_bytes: i64) -> i64 {
        // Rough estimate: ~1 second per MB
        let size_mb = file_size_bytes / (1024 * 1024);
        (size_mb + 1) * 1000
    }
}

impl Default for OcrService {
    fn default() -> Self {
        Self::new()
    }
}
