use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Timeline event
#[derive(Debug, Serialize)]
pub struct TimelineEvent {
    pub id: String,
    pub event_type: TimelineEventType,
    pub document_id: Option<uuid::Uuid>,
    pub document_title: Option<String>,
    pub user_id: i32,
    pub username: String,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

/// Timeline event type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEventType {
    DocumentCreated,
    DocumentUpdated,
    DocumentDeleted,
    DocumentViewed,
    DocumentShared,
    CommentAdded,
    CommentUpdated,
    TagAdded,
    TagRemoved,
    CategoryChanged,
    WorkflowStateChanged,
    AttachmentAdded,
    AttachmentRemoved,
    VersionCreated,
    VersionRestored,
    UserMentioned,
}

/// Timeline query parameters
#[derive(Debug, Deserialize, Default)]
pub struct TimelineQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub event_types: Option<Vec<TimelineEventType>>,
    pub document_id: Option<uuid::Uuid>,
    pub user_id: Option<i32>,
    pub category_id: Option<i32>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
}

/// Timeline response
#[derive(Debug, Serialize)]
pub struct TimelineResponse {
    pub events: Vec<TimelineEvent>,
    pub total: i64,
    pub page: i32,
    pub limit: i32,
    pub has_more: bool,
}

/// Activity summary by date
#[derive(Debug, Serialize)]
pub struct DailyActivity {
    pub date: NaiveDate,
    pub total_events: i64,
    pub documents_created: i64,
    pub documents_updated: i64,
    pub comments_added: i64,
    pub active_users: i64,
}

/// Activity heatmap data
#[derive(Debug, Serialize)]
pub struct ActivityHeatmap {
    pub data: Vec<HeatmapCell>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub max_value: i64,
}

/// Heatmap cell
#[derive(Debug, Serialize)]
pub struct HeatmapCell {
    pub date: NaiveDate,
    pub value: i64,
    pub level: i32, // 0-4 intensity level
}

/// User activity stream
#[derive(Debug, Serialize)]
pub struct UserActivityStream {
    pub user_id: i32,
    pub username: String,
    pub events: Vec<TimelineEvent>,
    pub total_events: i64,
    pub most_active_day: Option<NaiveDate>,
    pub streak_days: i32,
}

/// Document history
#[derive(Debug, Serialize)]
pub struct DocumentHistory {
    pub document_id: uuid::Uuid,
    pub title: String,
    pub events: Vec<TimelineEvent>,
    pub total_views: i64,
    pub total_edits: i64,
    pub contributors: Vec<ContributorInfo>,
}

/// Contributor info
#[derive(Debug, Serialize)]
pub struct ContributorInfo {
    pub user_id: i32,
    pub username: String,
    pub contribution_count: i64,
    pub last_contribution: DateTime<Utc>,
}

/// Timeline statistics
#[derive(Debug, Serialize)]
pub struct TimelineStats {
    pub total_events: i64,
    pub events_today: i64,
    pub events_this_week: i64,
    pub events_this_month: i64,
    pub by_type: Vec<EventTypeCount>,
    pub most_active_users: Vec<ActiveUserInfo>,
    pub most_active_documents: Vec<ActiveDocumentInfo>,
}

/// Event type count
#[derive(Debug, Serialize)]
pub struct EventTypeCount {
    pub event_type: TimelineEventType,
    pub count: i64,
}

/// Active user info
#[derive(Debug, Serialize)]
pub struct ActiveUserInfo {
    pub user_id: i32,
    pub username: String,
    pub event_count: i64,
}

/// Active document info
#[derive(Debug, Serialize)]
pub struct ActiveDocumentInfo {
    pub document_id: uuid::Uuid,
    pub title: String,
    pub event_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_query_default() {
        let query = TimelineQuery::default();
        assert!(query.page.is_none());
        assert!(query.limit.is_none());
        assert!(query.event_types.is_none());
        assert!(query.document_id.is_none());
        assert!(query.user_id.is_none());
        assert!(query.category_id.is_none());
        assert!(query.date_from.is_none());
        assert!(query.date_to.is_none());
    }

    #[test]
    fn test_timeline_event_type_serde_roundtrip() {
        let variants = vec![
            TimelineEventType::DocumentCreated,
            TimelineEventType::DocumentUpdated,
            TimelineEventType::DocumentDeleted,
            TimelineEventType::DocumentViewed,
            TimelineEventType::CommentAdded,
            TimelineEventType::TagAdded,
            TimelineEventType::TagRemoved,
            TimelineEventType::WorkflowStateChanged,
            TimelineEventType::VersionCreated,
            TimelineEventType::VersionRestored,
        ];

        for variant in variants {
            let serialized = serde_json::to_string(&variant).unwrap();
            let deserialized: TimelineEventType = serde_json::from_str(&serialized).unwrap();
            // Verify roundtrip by re-serializing
            let re_serialized = serde_json::to_string(&deserialized).unwrap();
            assert_eq!(serialized, re_serialized);
        }
    }

    #[test]
    fn test_timeline_event_type_snake_case_serialization() {
        let event_type = TimelineEventType::DocumentCreated;
        let serialized = serde_json::to_string(&event_type).unwrap();
        assert_eq!(serialized, "\"document_created\"");
    }

    #[test]
    fn test_timeline_event_type_workflow_state_changed_serialization() {
        let event_type = TimelineEventType::WorkflowStateChanged;
        let serialized = serde_json::to_string(&event_type).unwrap();
        assert_eq!(serialized, "\"workflow_state_changed\"");
    }

    #[test]
    fn test_timeline_event_type_user_mentioned_serialization() {
        let event_type = TimelineEventType::UserMentioned;
        let serialized = serde_json::to_string(&event_type).unwrap();
        assert_eq!(serialized, "\"user_mentioned\"");
    }

    #[test]
    fn test_timeline_response_has_more_logic() {
        // Simulating the has_more calculation used in timeline_service:
        // has_more = (offset + limit) < total
        let check_has_more = |offset: i32, limit: i32, total: i64| -> bool {
            (offset + limit) < total as i32
        };

        // Not at the end
        assert!(check_has_more(0, 10, 15));
        // Exactly at the end
        assert!(!check_has_more(0, 10, 10));
        // Past the end
        assert!(!check_has_more(10, 10, 15));
        // First page of many
        assert!(check_has_more(0, 50, 200));
        // Last page
        assert!(!check_has_more(190, 50, 200));
    }

    #[test]
    fn test_heatmap_cell_level_calculation() {
        // Simulating the level calculation from timeline_service::get_activity_heatmap:
        // level = ceil((value / max_value) * 4).min(4)
        let calc_level = |value: i64, max_value: i64| -> i32 {
            ((value as f32 / max_value as f32) * 4.0).ceil() as i32
        };

        let max = 100i64;
        // 0% activity => level 0
        assert_eq!(calc_level(0, max), 0);
        // 25% activity => level 1
        assert_eq!(calc_level(25, max), 1);
        // 50% activity => level 2
        assert_eq!(calc_level(50, max), 2);
        // 75% activity => level 3
        assert_eq!(calc_level(75, max), 3);
        // 100% activity => level 4
        assert_eq!(calc_level(100, max), 4);
    }

    #[test]
    fn test_daily_activity_struct_construction() {
        use chrono::NaiveDate;
        let date = NaiveDate::from_ymd_opt(2026, 2, 19).unwrap();
        let activity = DailyActivity {
            date,
            total_events: 42,
            documents_created: 5,
            documents_updated: 10,
            comments_added: 15,
            active_users: 3,
        };

        assert_eq!(activity.total_events, 42);
        assert_eq!(activity.documents_created, 5);
        assert_eq!(activity.documents_updated, 10);
        assert_eq!(activity.comments_added, 15);
        assert_eq!(activity.active_users, 3);
        // Sum of subtypes
        assert_eq!(
            activity.documents_created + activity.documents_updated + activity.comments_added,
            30
        );
    }
}
