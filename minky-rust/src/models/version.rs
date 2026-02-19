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

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DiffOperation {
    Add,
    Remove,
    Keep,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_operation_serde_add() {
        let op = DiffOperation::Add;
        let serialized = serde_json::to_string(&op).unwrap();
        assert_eq!(serialized, "\"add\"");
    }

    #[test]
    fn test_diff_operation_serde_remove() {
        let op = DiffOperation::Remove;
        let serialized = serde_json::to_string(&op).unwrap();
        assert_eq!(serialized, "\"remove\"");
    }

    #[test]
    fn test_diff_operation_serde_keep() {
        let op = DiffOperation::Keep;
        let serialized = serde_json::to_string(&op).unwrap();
        assert_eq!(serialized, "\"keep\"");
    }

    #[test]
    fn test_version_diff_additions_deletions() {
        let diff = VersionDiff {
            from_version: 1,
            to_version: 2,
            additions: 10,
            deletions: 3,
            diff_lines: vec![],
        };

        assert_eq!(diff.from_version, 1);
        assert_eq!(diff.to_version, 2);
        assert_eq!(diff.additions, 10);
        assert_eq!(diff.deletions, 3);
        assert!(diff.diff_lines.is_empty());
    }

    #[test]
    fn test_diff_line_construction() {
        let line = DiffLine {
            line_number: 42,
            operation: DiffOperation::Add,
            content: "fn new_function() {}".to_string(),
        };

        assert_eq!(line.line_number, 42);
        assert_eq!(line.operation, DiffOperation::Add);
        assert_eq!(line.content, "fn new_function() {}");
    }

    #[test]
    fn test_version_diff_from_to_ordering() {
        // to_version must be greater than from_version in normal usage
        let diff = VersionDiff {
            from_version: 3,
            to_version: 7,
            additions: 0,
            deletions: 0,
            diff_lines: vec![],
        };
        assert!(diff.to_version > diff.from_version);
    }

    #[test]
    fn test_version_diff_net_change() {
        let diff = VersionDiff {
            from_version: 1,
            to_version: 2,
            additions: 15,
            deletions: 8,
            diff_lines: vec![],
        };
        // Net change: additions - deletions
        let net = diff.additions - diff.deletions;
        assert_eq!(net, 7);
    }
}
