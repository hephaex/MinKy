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
#[derive(Debug, Deserialize)]
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
