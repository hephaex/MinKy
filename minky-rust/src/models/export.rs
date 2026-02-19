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

    #[test]
    fn test_export_format_all_variants_serialize() {
        let formats = [
            (ExportFormat::Json, "\"json\""),
            (ExportFormat::Csv, "\"csv\""),
            (ExportFormat::Markdown, "\"markdown\""),
            (ExportFormat::Html, "\"html\""),
            (ExportFormat::Pdf, "\"pdf\""),
            (ExportFormat::Docx, "\"docx\""),
            (ExportFormat::Org, "\"org\""),
        ];
        for (fmt, expected) in formats {
            let json = serde_json::to_string(&fmt).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn test_export_status_all_variants_serialize() {
        let statuses = [
            (ExportStatus::Pending, "\"pending\""),
            (ExportStatus::Processing, "\"processing\""),
            (ExportStatus::Completed, "\"completed\""),
            (ExportStatus::Failed, "\"failed\""),
        ];
        for (status, expected) in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn test_merge_strategy_default_is_skip() {
        assert!(matches!(MergeStrategy::default(), MergeStrategy::Skip));
    }

    #[test]
    fn test_merge_strategy_all_variants_serialize() {
        let strategies = [
            (MergeStrategy::Skip, "\"skip\""),
            (MergeStrategy::Replace, "\"replace\""),
            (MergeStrategy::Merge, "\"merge\""),
            (MergeStrategy::CreateNew, "\"create_new\""),
        ];
        for (strategy, expected) in strategies {
            let json = serde_json::to_string(&strategy).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn test_export_request_all_optional_fields_default_none() {
        let req = ExportRequest {
            document_ids: None,
            category_id: None,
            format: None,
            include_metadata: None,
            include_attachments: None,
            include_comments: None,
            include_versions: None,
        };
        assert!(req.document_ids.is_none());
        assert!(req.category_id.is_none());
        assert!(req.format.is_none());
        assert!(req.include_metadata.is_none());
        assert!(req.include_attachments.is_none());
        assert!(req.include_comments.is_none());
        assert!(req.include_versions.is_none());
    }

    #[test]
    fn test_exported_document_serde_roundtrip() {
        use chrono::Utc;
        use uuid::Uuid;
        let doc = ExportedDocument {
            id: Uuid::new_v4(),
            title: "Export Test".to_string(),
            content: "Some content".to_string(),
            category_name: Some("Tech".to_string()),
            tags: vec!["rust".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: Some(serde_json::json!({"key": "value"})),
        };
        let json = serde_json::to_string(&doc).unwrap();
        let back: ExportedDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, "Export Test");
        assert_eq!(back.category_name, Some("Tech".to_string()));
        assert_eq!(back.tags, vec!["rust".to_string()]);
    }

    #[test]
    fn test_import_error_serializes() {
        let err = ImportError {
            item_index: 3,
            item_name: Some("doc.md".to_string()),
            error_message: "parse error".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"item_index\":3"));
        assert!(json.contains("\"item_name\":\"doc.md\""));
        assert!(json.contains("parse error"));
    }

    #[test]
    fn test_import_error_no_name_serializes_null() {
        let err = ImportError {
            item_index: 0,
            item_name: None,
            error_message: "unknown error".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"item_name\":null"));
    }
}
