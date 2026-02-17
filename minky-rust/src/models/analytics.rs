use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Document analytics overview
#[derive(Debug, Serialize)]
pub struct AnalyticsOverview {
    pub total_documents: i64,
    pub total_views: i64,
    pub total_comments: i64,
    pub total_users: i64,
    pub documents_this_period: i64,
    pub views_this_period: i64,
    pub active_users: i64,
    pub avg_document_length: f64,
}

/// Time series data point
#[derive(Debug, Serialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: i64,
}

/// Document activity metrics
#[derive(Debug, Serialize)]
pub struct DocumentMetrics {
    pub document_id: String,
    pub title: String,
    pub view_count: i64,
    pub comment_count: i64,
    pub version_count: i64,
    pub last_viewed: Option<DateTime<Utc>>,
    pub last_edited: DateTime<Utc>,
    pub engagement_score: f64,
}

/// User activity metrics
#[derive(Debug, Serialize)]
pub struct UserMetrics {
    pub user_id: i32,
    pub username: String,
    pub documents_created: i64,
    pub comments_made: i64,
    pub edits_made: i64,
    pub last_active: DateTime<Utc>,
    pub activity_score: f64,
}

/// Category statistics
#[derive(Debug, Serialize)]
pub struct CategoryStats {
    pub category_id: i32,
    pub category_name: String,
    pub document_count: i64,
    pub total_views: i64,
    pub avg_document_length: f64,
}

/// Tag usage statistics
#[derive(Debug, Serialize)]
pub struct TagStats {
    pub tag_name: String,
    pub usage_count: i64,
    pub trend: TrendDirection,
    pub related_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
}

/// Content analysis metrics
#[derive(Debug, Serialize)]
pub struct ContentAnalysis {
    pub word_count: i64,
    pub avg_sentence_length: f64,
    pub reading_time_minutes: i32,
    pub complexity_score: f64,
    pub top_keywords: Vec<KeywordCount>,
    pub sentiment: SentimentScore,
}

#[derive(Debug, Serialize)]
pub struct KeywordCount {
    pub keyword: String,
    pub count: i64,
    pub tfidf_score: f64,
}

#[derive(Debug, Serialize)]
pub struct SentimentScore {
    pub positive: f64,
    pub negative: f64,
    pub neutral: f64,
    pub overall: String,
}

/// Workflow analytics
#[derive(Debug, Serialize)]
pub struct WorkflowAnalytics {
    pub total_workflows: i64,
    pub by_status: HashMap<String, i64>,
    pub avg_completion_time_hours: f64,
    pub overdue_count: i64,
    pub completion_rate: f64,
}

/// Dashboard data
#[derive(Debug, Serialize)]
pub struct DashboardData {
    pub overview: AnalyticsOverview,
    pub recent_documents: Vec<DocumentMetrics>,
    pub top_viewed: Vec<DocumentMetrics>,
    pub active_users: Vec<UserMetrics>,
    pub category_breakdown: Vec<CategoryStats>,
    pub activity_timeline: Vec<TimeSeriesPoint>,
}

/// Report generation request
#[derive(Debug, Deserialize)]
pub struct ReportRequest {
    pub report_type: ReportType,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub format: ReportFormat,
    pub include_charts: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    Overview,
    UserActivity,
    ContentAnalysis,
    WorkflowStatus,
    SearchAnalytics,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    Json,
    Csv,
    Pdf,
}

/// Search analytics
#[derive(Debug, Serialize)]
pub struct SearchAnalytics {
    pub total_searches: i64,
    pub unique_queries: i64,
    pub avg_results_per_search: f64,
    pub zero_result_rate: f64,
    pub top_queries: Vec<QueryStats>,
    pub search_trends: Vec<TimeSeriesPoint>,
}

#[derive(Debug, Serialize)]
pub struct QueryStats {
    pub query: String,
    pub count: i64,
    pub avg_click_position: f64,
    pub zero_results: bool,
}
