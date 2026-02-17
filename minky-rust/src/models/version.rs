use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Document version model representing the document_versions table
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Version {
    pub id: i32,
    pub document_id: Uuid,
    pub content: String,
    pub version_number: i32,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
}

/// DTO for creating a new version
#[derive(Debug, Deserialize)]
pub struct CreateVersion {
    pub document_id: Uuid,
    pub content: String,
}

/// Version with author information
#[derive(Debug, Serialize, FromRow)]
pub struct VersionWithAuthor {
    pub id: i32,
    pub document_id: Uuid,
    pub content: String,
    pub version_number: i32,
    pub created_by: i32,
    pub author_name: String,
    pub created_at: DateTime<Utc>,
}

/// Version diff between two versions
#[derive(Debug, Serialize)]
pub struct VersionDiff {
    pub from_version: i32,
    pub to_version: i32,
    pub additions: i32,
    pub deletions: i32,
    pub diff_lines: Vec<DiffLine>,
}

#[derive(Debug, Serialize)]
pub struct DiffLine {
    pub line_number: i32,
    pub operation: DiffOperation,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffOperation {
    Add,
    Remove,
    Keep,
}
