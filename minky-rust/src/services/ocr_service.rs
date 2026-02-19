use anyhow::Result;
use chrono::Utc;
use std::process::Command;

use crate::models::{
    BlockType, BoundingBox, OcrEngine, OcrJob, OcrMetadata, OcrPage, OcrRequest,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported_format_pdf() {
        let svc = OcrService::new();
        assert!(svc.is_supported_format("document.pdf"));
    }

    #[test]
    fn test_is_supported_format_png() {
        let svc = OcrService::new();
        assert!(svc.is_supported_format("image.PNG"), "Extension check should be case-insensitive");
    }

    #[test]
    fn test_is_supported_format_jpg() {
        let svc = OcrService::new();
        assert!(svc.is_supported_format("photo.jpg"));
    }

    #[test]
    fn test_is_supported_format_tiff() {
        let svc = OcrService::new();
        assert!(svc.is_supported_format("scan.tiff"));
    }

    #[test]
    fn test_is_supported_format_unsupported_returns_false() {
        let svc = OcrService::new();
        assert!(!svc.is_supported_format("document.docx"), "Unsupported format should return false");
    }

    #[test]
    fn test_is_supported_format_no_extension_returns_false() {
        let svc = OcrService::new();
        assert!(!svc.is_supported_format("Makefile"), "File without extension should return false");
    }

    #[test]
    fn test_estimate_processing_time_small_file() {
        let svc = OcrService::new();
        // Under 1MB: (0 + 1) * 1000 = 1000ms
        let estimate = svc.estimate_processing_time(512 * 1024);
        assert_eq!(estimate, 1000);
    }

    #[test]
    fn test_estimate_processing_time_one_mb() {
        let svc = OcrService::new();
        // Exactly 1MB: (1 + 1) * 1000 = 2000ms
        let estimate = svc.estimate_processing_time(1024 * 1024);
        assert_eq!(estimate, 2000);
    }

    #[test]
    fn test_estimate_processing_time_zero_bytes() {
        let svc = OcrService::new();
        // 0 bytes: (0 + 1) * 1000 = 1000ms (minimum 1 second)
        let estimate = svc.estimate_processing_time(0);
        assert_eq!(estimate, 1000);
    }

    #[test]
    fn test_estimate_processing_time_scales_with_size() {
        let svc = OcrService::new();
        let small = svc.estimate_processing_time(1024 * 1024);
        let large = svc.estimate_processing_time(10 * 1024 * 1024);
        assert!(large > small, "Larger file should take longer to estimate");
    }

    #[test]
    fn test_get_settings_returns_defaults() {
        let svc = OcrService::new();
        let settings = svc.get_settings();
        assert!(!settings.supported_formats.is_empty(), "Supported formats should not be empty");
        assert!(settings.max_file_size_mb > 0, "Max file size should be positive");
    }

    #[test]
    fn test_update_settings_changes_max_file_size() {
        let mut svc = OcrService::new();
        let new_settings = OcrSettings {
            default_engine: crate::models::OcrEngine::Tesseract,
            default_languages: vec!["eng".to_string()],
            auto_ocr_on_upload: true,
            max_file_size_mb: 100,
            supported_formats: vec!["pdf".to_string()],
        };
        svc.update_settings(new_settings);
        assert_eq!(svc.get_settings().max_file_size_mb, 100);
        assert!(svc.get_settings().auto_ocr_on_upload);
    }
}
