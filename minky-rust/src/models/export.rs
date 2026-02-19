use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Export format
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    #[default]
    Json,
    Csv,
    Markdown,
    Html,
    Pdf,
    Docx,
    Org,
}

/// Export request
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub document_ids: Option<Vec<uuid::Uuid>>,
    pub category_id: Option<i32>,
    pub format: Option<ExportFormat>,
    pub include_metadata: Option<bool>,
    pub include_attachments: Option<bool>,
    pub include_comments: Option<bool>,
    pub include_versions: Option<bool>,
}

/// Export job status
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExportStatus {
    #[default]
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Export job
#[derive(Debug, Serialize)]
pub struct ExportJob {
    pub id: String,
    pub user_id: i32,
    pub format: ExportFormat,
    pub status: ExportStatus,
    pub document_count: i32,
    pub progress_percent: i32,
    pub download_url: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Import request
#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    pub format: Option<ExportFormat>,
    pub category_id: Option<i32>,
    pub merge_strategy: Option<MergeStrategy>,
}

/// Merge strategy for imports
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    #[default]
    Skip,
    Replace,
    Merge,
    CreateNew,
}

/// Import job
#[derive(Debug, Serialize)]
pub struct ImportJob {
    pub id: String,
    pub user_id: i32,
    pub status: ExportStatus,
    pub total_items: i32,
    pub processed_items: i32,
    pub success_count: i32,
    pub error_count: i32,
    pub errors: Vec<ImportError>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Import error
#[derive(Debug, Serialize)]
pub struct ImportError {
    pub item_index: i32,
    pub item_name: Option<String>,
    pub error_message: String,
}

/// Exported document
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportedDocument {
    pub id: uuid::Uuid,
    pub title: String,
    pub content: String,
    pub category_name: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_default_is_json() {
        assert!(matches!(ExportFormat::default(), ExportFormat::Json));
    }

    #[test]
    fn test_export_status_default_is_pending() {
        assert!(matches!(ExportStatus::default(), ExportStatus::Pending));
    }
}
