use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::models::{
    AnalyticsOverview, CategoryStats, ContentAnalysis, DashboardData, DocumentMetrics,
    KeywordCount, SentimentScore, TagStats, TimeSeriesPoint, TrendDirection, UserMetrics,
    WorkflowAnalytics,
};

/// Analytics service for metrics and reporting
pub struct AnalyticsService {
    db: PgPool,
}

impl AnalyticsService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Get analytics overview
    pub async fn get_overview(&self, days: i32) -> Result<AnalyticsOverview> {
        let cutoff = Utc::now() - Duration::days(days as i64);

        let total_documents: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents")
            .fetch_one(&self.db)
            .await?;

        let total_views: (i64,) = sqlx::query_as("SELECT COALESCE(SUM(view_count), 0) FROM documents")
            .fetch_one(&self.db)
            .await?;

        let total_comments: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM comments")
            .fetch_one(&self.db)
            .await?;

        let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_active = true")
            .fetch_one(&self.db)
            .await?;

        let documents_this_period: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM documents WHERE created_at >= $1"
        )
        .bind(cutoff)
        .fetch_one(&self.db)
        .await?;

        let views_this_period: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(view_count), 0) FROM documents WHERE updated_at >= $1"
        )
        .bind(cutoff)
        .fetch_one(&self.db)
        .await?;

        let active_users: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT user_id) FROM (
                SELECT user_id FROM documents WHERE created_at >= $1
                UNION
                SELECT user_id FROM comments WHERE created_at >= $1
            ) AS active
            "#
        )
        .bind(cutoff)
        .fetch_one(&self.db)
        .await?;

        let avg_length: (Option<f64>,) = sqlx::query_as(
            "SELECT AVG(LENGTH(content)) FROM documents"
        )
        .fetch_one(&self.db)
        .await?;

        Ok(AnalyticsOverview {
            total_documents: total_documents.0,
            total_views: total_views.0,
            total_comments: total_comments.0,
            total_users: total_users.0,
            documents_this_period: documents_this_period.0,
            views_this_period: views_this_period.0,
            active_users: active_users.0,
            avg_document_length: avg_length.0.unwrap_or(0.0),
        })
    }

    /// Get top viewed documents
    pub async fn get_top_documents(&self, limit: i32) -> Result<Vec<DocumentMetrics>> {
        let rows: Vec<(String, String, i64, i64, i64, Option<DateTime<Utc>>, DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT
                d.id::text,
                d.title,
                d.view_count::bigint,
                (SELECT COUNT(*) FROM comments WHERE document_id = d.id)::bigint as comment_count,
                (SELECT COUNT(*) FROM document_versions WHERE document_id = d.id)::bigint as version_count,
                d.updated_at as last_viewed,
                d.updated_at as last_edited
            FROM documents d
            ORDER BY d.view_count DESC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(|r| DocumentMetrics {
            document_id: r.0,
            title: r.1,
            view_count: r.2,
            comment_count: r.3,
            version_count: r.4,
            last_viewed: r.5,
            last_edited: r.6,
            engagement_score: calculate_engagement(r.2, r.3, r.4),
        }).collect())
    }

    /// Get most active users
    pub async fn get_active_users(&self, limit: i32) -> Result<Vec<UserMetrics>> {
        let rows: Vec<(i32, String, i64, i64, i64, DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT
                u.id,
                u.username,
                (SELECT COUNT(*) FROM documents WHERE user_id = u.id)::bigint as docs_created,
                (SELECT COUNT(*) FROM comments WHERE user_id = u.id)::bigint as comments_made,
                (SELECT COUNT(*) FROM document_versions WHERE created_by = u.id)::bigint as edits_made,
                COALESCE(
                    (SELECT MAX(created_at) FROM documents WHERE user_id = u.id),
                    u.created_at
                ) as last_active
            FROM users u
            WHERE u.is_active = true
            ORDER BY docs_created + comments_made + edits_made DESC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(|r| UserMetrics {
            user_id: r.0,
            username: r.1,
            documents_created: r.2,
            comments_made: r.3,
            edits_made: r.4,
            last_active: r.5,
            activity_score: (r.2 * 10 + r.3 * 5 + r.4 * 3) as f64,
        }).collect())
    }

    /// Get category statistics
    pub async fn get_category_stats(&self) -> Result<Vec<CategoryStats>> {
        let rows: Vec<(i32, String, i64, i64, Option<f64>)> = sqlx::query_as(
            r#"
            SELECT
                c.id,
                c.name,
                COUNT(d.id)::bigint as document_count,
                COALESCE(SUM(d.view_count), 0)::bigint as total_views,
                AVG(LENGTH(d.content)) as avg_length
            FROM categories c
            LEFT JOIN documents d ON c.id = d.category_id
            GROUP BY c.id, c.name
            ORDER BY document_count DESC
            "#
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(|r| CategoryStats {
            category_id: r.0,
            category_name: r.1,
            document_count: r.2,
            total_views: r.3,
            avg_document_length: r.4.unwrap_or(0.0),
        }).collect())
    }

    /// Get tag statistics
    pub async fn get_tag_stats(&self, limit: i32) -> Result<Vec<TagStats>> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT t.name, COUNT(dt.document_id)::bigint as usage_count
            FROM tags t
            JOIN document_tags dt ON t.id = dt.tag_id
            GROUP BY t.name
            ORDER BY usage_count DESC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(|r| TagStats {
            tag_name: r.0,
            usage_count: r.1,
            trend: TrendDirection::Stable,
            related_tags: vec![],
        }).collect())
    }

    /// Get activity timeline
    pub async fn get_activity_timeline(&self, days: i32) -> Result<Vec<TimeSeriesPoint>> {
        let rows: Vec<(DateTime<Utc>, i64)> = sqlx::query_as(
            r#"
            SELECT
                DATE_TRUNC('day', created_at) as day,
                COUNT(*)::bigint as count
            FROM documents
            WHERE created_at >= NOW() - INTERVAL '1 day' * $1
            GROUP BY day
            ORDER BY day
            "#
        )
        .bind(days)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(|r| TimeSeriesPoint {
            timestamp: r.0,
            value: r.1,
        }).collect())
    }

    /// Get workflow analytics
    pub async fn get_workflow_analytics(&self) -> Result<WorkflowAnalytics> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workflows")
            .fetch_one(&self.db)
            .await?;

        let by_status_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT status, COUNT(*)::bigint FROM workflows GROUP BY status"
        )
        .fetch_all(&self.db)
        .await?;

        let by_status: HashMap<String, i64> = by_status_rows.into_iter().collect();

        let overdue: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM workflows WHERE due_date < NOW() AND status NOT IN ('published', 'archived')"
        )
        .fetch_one(&self.db)
        .await?;

        let completed = by_status.get("published").copied().unwrap_or(0) +
                        by_status.get("archived").copied().unwrap_or(0);
        let completion_rate = if total.0 > 0 {
            completed as f64 / total.0 as f64 * 100.0
        } else {
            0.0
        };

        Ok(WorkflowAnalytics {
            total_workflows: total.0,
            by_status,
            avg_completion_time_hours: 0.0, // Would require more complex query
            overdue_count: overdue.0,
            completion_rate,
        })
    }

    /// Get dashboard data
    pub async fn get_dashboard(&self) -> Result<DashboardData> {
        let overview = self.get_overview(30).await?;
        let recent_documents = self.get_recent_documents(10).await?;
        let top_viewed = self.get_top_documents(10).await?;
        let active_users = self.get_active_users(10).await?;
        let category_breakdown = self.get_category_stats().await?;
        let activity_timeline = self.get_activity_timeline(30).await?;

        Ok(DashboardData {
            overview,
            recent_documents,
            top_viewed,
            active_users,
            category_breakdown,
            activity_timeline,
        })
    }

    /// Get recent documents
    async fn get_recent_documents(&self, limit: i32) -> Result<Vec<DocumentMetrics>> {
        let rows: Vec<(String, String, i64, i64, i64, Option<DateTime<Utc>>, DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT
                d.id::text,
                d.title,
                d.view_count::bigint,
                (SELECT COUNT(*) FROM comments WHERE document_id = d.id)::bigint,
                (SELECT COUNT(*) FROM document_versions WHERE document_id = d.id)::bigint,
                d.updated_at,
                d.updated_at
            FROM documents d
            ORDER BY d.created_at DESC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(|r| DocumentMetrics {
            document_id: r.0,
            title: r.1,
            view_count: r.2,
            comment_count: r.3,
            version_count: r.4,
            last_viewed: r.5,
            last_edited: r.6,
            engagement_score: calculate_engagement(r.2, r.3, r.4),
        }).collect())
    }

    /// Analyze content for metrics
    pub fn analyze_content(content: &str) -> ContentAnalysis {
        let words: Vec<&str> = content.split_whitespace().collect();
        let word_count = words.len() as i64;

        let sentences: Vec<&str> = content.split(&['.', '!', '?'][..]).collect();
        let sentence_count = sentences.len().max(1);
        let avg_sentence_length = word_count as f64 / sentence_count as f64;

        let reading_time = (word_count as f64 / 200.0).ceil() as i32;

        // Simple complexity based on average word length
        let avg_word_length: f64 = words.iter().map(|w| w.len()).sum::<usize>() as f64 / words.len().max(1) as f64;
        let complexity_score = (avg_word_length / 10.0 * 100.0).min(100.0);

        // Simple keyword extraction (top words by frequency)
        let mut word_freq: HashMap<String, i64> = HashMap::new();
        for word in &words {
            let lower = word.to_lowercase();
            if lower.len() > 3 {
                *word_freq.entry(lower).or_insert(0) += 1;
            }
        }

        let mut top_keywords: Vec<KeywordCount> = word_freq.into_iter()
            .map(|(k, v)| KeywordCount {
                keyword: k,
                count: v,
                tfidf_score: v as f64,
            })
            .collect();

        top_keywords.sort_by(|a, b| b.count.cmp(&a.count));
        top_keywords.truncate(10);

        ContentAnalysis {
            word_count,
            avg_sentence_length,
            reading_time_minutes: reading_time,
            complexity_score,
            top_keywords,
            sentiment: SentimentScore {
                positive: 0.33,
                negative: 0.33,
                neutral: 0.34,
                overall: "neutral".to_string(),
            },
        }
    }
}

fn calculate_engagement(views: i64, comments: i64, versions: i64) -> f64 {
    (views as f64 * 1.0) + (comments as f64 * 5.0) + (versions as f64 * 3.0)
}
