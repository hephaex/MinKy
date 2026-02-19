use anyhow::Result;
use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;

use crate::models::{
    ActivityHeatmap, ContributorInfo, DailyActivity,
    DocumentHistory, HeatmapCell, TimelineEvent, TimelineEventType, TimelineQuery,
    TimelineResponse, TimelineStats, UserActivityStream,
};

// ---------------------------------------------------------------------------
// Pure helpers (no I/O – unit-testable)
// ---------------------------------------------------------------------------

/// Compute the current streak (consecutive days with activity) from an
/// ordered list of active days (most-recent first).
///
/// Rules:
/// - If the list is empty, streak = 0.
/// - If the most-recent active day is neither today nor yesterday, streak = 0.
/// - Otherwise, count consecutive days going backward.
pub fn compute_streak_from_days(active_days: &[NaiveDate], today: NaiveDate) -> i32 {
    if active_days.is_empty() {
        return 0;
    }

    let most_recent = active_days[0];
    if most_recent != today && most_recent != today - Duration::days(1) {
        return 0;
    }

    let mut streak = 1i32;
    for i in 0..active_days.len() - 1 {
        let current = active_days[i];
        let next = active_days[i + 1];
        if current - next == Duration::days(1) {
            streak += 1;
        } else {
            break;
        }
    }
    streak
}

/// Compute the heatmap level (0–4) for a cell given the max value across all cells.
/// Returns 0 when `max_value` is 0 (avoid division by zero).
pub fn compute_heatmap_level(cell_value: i64, max_value: i64) -> i32 {
    if max_value == 0 {
        return 0;
    }
    let level = ((cell_value as f32 / max_value as f32) * 4.0).ceil() as i32;
    level.min(4)
}

/// Raw DB row type for timeline event queries
type TimelineEventRow = (
    String,
    String,
    Option<uuid::Uuid>,
    Option<String>,
    i32,
    String,
    String,
    Option<serde_json::Value>,
    chrono::DateTime<chrono::Utc>,
);

/// Timeline service for activity tracking
pub struct TimelineService {
    db: PgPool,
}

impl TimelineService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Get timeline events
    pub async fn get_timeline(&self, query: TimelineQuery) -> Result<TimelineResponse> {
        let page = query.page.unwrap_or(1);
        let limit = query.limit.unwrap_or(50).min(200);
        let offset = (page - 1) * limit;

        let rows: Vec<TimelineEventRow> = sqlx::query_as(
            r#"
            SELECT
                e.id,
                e.event_type,
                e.document_id,
                d.title,
                e.user_id,
                u.username,
                e.description,
                e.metadata,
                e.created_at
            FROM timeline_events e
            LEFT JOIN documents d ON e.document_id = d.id
            LEFT JOIN users u ON e.user_id = u.id
            WHERE ($1::uuid IS NULL OR e.document_id = $1)
              AND ($2::int IS NULL OR e.user_id = $2)
              AND ($3::int IS NULL OR d.category_id = $3)
              AND ($4::timestamptz IS NULL OR e.created_at >= $4)
              AND ($5::timestamptz IS NULL OR e.created_at <= $5)
            ORDER BY e.created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(query.document_id)
        .bind(query.user_id)
        .bind(query.category_id)
        .bind(query.date_from)
        .bind(query.date_to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        let events: Vec<TimelineEvent> = rows
            .into_iter()
            .map(|r| TimelineEvent {
                id: r.0,
                event_type: serde_json::from_str(&r.1).unwrap_or(TimelineEventType::DocumentViewed),
                document_id: r.2,
                document_title: r.3,
                user_id: r.4,
                username: r.5,
                description: r.6,
                metadata: r.7,
                timestamp: r.8,
            })
            .collect();

        // Get total count
        let total: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM timeline_events e
            LEFT JOIN documents d ON e.document_id = d.id
            WHERE ($1::uuid IS NULL OR e.document_id = $1)
              AND ($2::int IS NULL OR e.user_id = $2)
              AND ($3::int IS NULL OR d.category_id = $3)
            "#,
        )
        .bind(query.document_id)
        .bind(query.user_id)
        .bind(query.category_id)
        .fetch_one(&self.db)
        .await?;

        let has_more = (offset + limit) < total.0 as i32;

        Ok(TimelineResponse {
            events,
            total: total.0,
            page,
            limit,
            has_more,
        })
    }

    /// Log timeline event
    pub async fn log_event(
        &self,
        event_type: TimelineEventType,
        user_id: i32,
        document_id: Option<uuid::Uuid>,
        description: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<String> {
        let event_id = uuid::Uuid::new_v4().to_string();
        let event_type_str = serde_json::to_string(&event_type)?;

        sqlx::query(
            r#"
            INSERT INTO timeline_events (id, event_type, user_id, document_id, description, metadata, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            "#,
        )
        .bind(&event_id)
        .bind(&event_type_str)
        .bind(user_id)
        .bind(document_id)
        .bind(description)
        .bind(metadata)
        .execute(&self.db)
        .await?;

        Ok(event_id)
    }

    /// Get daily activity summary
    pub async fn get_daily_activity(&self, days: i32) -> Result<Vec<DailyActivity>> {
        let start_date = Utc::now() - Duration::days(days as i64);

        let rows: Vec<(
            chrono::DateTime<chrono::Utc>,
            i64,
            i64,
            i64,
            i64,
            i64,
        )> = sqlx::query_as(
            r#"
            SELECT
                DATE_TRUNC('day', e.created_at) as date,
                COUNT(*)::bigint as total_events,
                COUNT(*) FILTER (WHERE e.event_type = '"document_created"')::bigint as docs_created,
                COUNT(*) FILTER (WHERE e.event_type = '"document_updated"')::bigint as docs_updated,
                COUNT(*) FILTER (WHERE e.event_type = '"comment_added"')::bigint as comments_added,
                COUNT(DISTINCT e.user_id)::bigint as active_users
            FROM timeline_events e
            WHERE e.created_at >= $1
            GROUP BY date
            ORDER BY date
            "#,
        )
        .bind(start_date)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| DailyActivity {
                date: r.0.date_naive(),
                total_events: r.1,
                documents_created: r.2,
                documents_updated: r.3,
                comments_added: r.4,
                active_users: r.5,
            })
            .collect())
    }

    /// Get activity heatmap
    pub async fn get_activity_heatmap(&self, year: i32) -> Result<ActivityHeatmap> {
        let start_date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

        let rows: Vec<(chrono::DateTime<chrono::Utc>, i64)> = sqlx::query_as(
            r#"
            SELECT DATE_TRUNC('day', created_at) as date, COUNT(*)::bigint
            FROM timeline_events
            WHERE created_at >= $1 AND created_at <= $2
            GROUP BY date
            "#,
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.db)
        .await?;

        let max_value = rows.iter().map(|r| r.1).max().unwrap_or(1);

        let data: Vec<HeatmapCell> = rows
            .into_iter()
            .map(|r| HeatmapCell {
                date: r.0.date_naive(),
                value: r.1,
                level: compute_heatmap_level(r.1, max_value),
            })
            .collect();

        Ok(ActivityHeatmap {
            data,
            start_date,
            end_date,
            max_value,
        })
    }

    /// Get user activity stream
    pub async fn get_user_activity(&self, user_id: i32, limit: i32) -> Result<UserActivityStream> {
        let username: (String,) = sqlx::query_as("SELECT username FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.db)
            .await?;

        let events = self
            .get_timeline(TimelineQuery {
                user_id: Some(user_id),
                limit: Some(limit),
                ..Default::default()
            })
            .await?;

        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM timeline_events WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.db)
                .await?;

        // Get most active day
        let most_active: Option<(chrono::DateTime<chrono::Utc>,)> = sqlx::query_as(
            r#"
            SELECT DATE_TRUNC('day', created_at) as date
            FROM timeline_events
            WHERE user_id = $1
            GROUP BY date
            ORDER BY COUNT(*) DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        // Calculate streak
        let streak_days = self.calculate_streak(user_id).await.unwrap_or(0);

        Ok(UserActivityStream {
            user_id,
            username: username.0,
            events: events.events,
            total_events: total.0,
            most_active_day: most_active.map(|r| r.0.date_naive()),
            streak_days,
        })
    }

    /// Calculate user streak
    async fn calculate_streak(&self, user_id: i32) -> Result<i32> {
        let rows: Vec<(chrono::DateTime<chrono::Utc>,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT DATE_TRUNC('day', created_at) as date
            FROM timeline_events
            WHERE user_id = $1
            ORDER BY date DESC
            LIMIT 365
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        let today = Utc::now().date_naive();
        let active_days: Vec<NaiveDate> = rows.iter().map(|r| r.0.date_naive()).collect();
        Ok(compute_streak_from_days(&active_days, today))
    }

    /// Get document history
    pub async fn get_document_history(&self, document_id: uuid::Uuid) -> Result<DocumentHistory> {
        let title: (String,) = sqlx::query_as("SELECT title FROM documents WHERE id = $1")
            .bind(document_id)
            .fetch_one(&self.db)
            .await?;

        let events = self
            .get_timeline(TimelineQuery {
                document_id: Some(document_id),
                limit: Some(100),
                ..Default::default()
            })
            .await?;

        let stats: (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE event_type = '"document_viewed"')::bigint as views,
                COUNT(*) FILTER (WHERE event_type = '"document_updated"')::bigint as edits
            FROM timeline_events
            WHERE document_id = $1
            "#,
        )
        .bind(document_id)
        .fetch_one(&self.db)
        .await?;

        let contributor_rows: Vec<(i32, String, i64, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as(
                r#"
            SELECT e.user_id, u.username, COUNT(*)::bigint, MAX(e.created_at)
            FROM timeline_events e
            JOIN users u ON e.user_id = u.id
            WHERE e.document_id = $1
            GROUP BY e.user_id, u.username
            ORDER BY COUNT(*) DESC
            "#,
            )
            .bind(document_id)
            .fetch_all(&self.db)
            .await?;

        let contributors: Vec<ContributorInfo> = contributor_rows
            .into_iter()
            .map(|r| ContributorInfo {
                user_id: r.0,
                username: r.1,
                contribution_count: r.2,
                last_contribution: r.3,
            })
            .collect();

        Ok(DocumentHistory {
            document_id,
            title: title.0,
            events: events.events,
            total_views: stats.0,
            total_edits: stats.1,
            contributors,
        })
    }

    /// Get timeline statistics
    pub async fn get_stats(&self) -> Result<TimelineStats> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM timeline_events")
            .fetch_one(&self.db)
            .await?;

        let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
        let week_start = Utc::now() - Duration::days(7);
        let month_start = Utc::now() - Duration::days(30);

        let today: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM timeline_events WHERE created_at >= $1",
        )
        .bind(today_start)
        .fetch_one(&self.db)
        .await?;

        let this_week: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM timeline_events WHERE created_at >= $1",
        )
        .bind(week_start)
        .fetch_one(&self.db)
        .await?;

        let this_month: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM timeline_events WHERE created_at >= $1",
        )
        .bind(month_start)
        .fetch_one(&self.db)
        .await?;

        Ok(TimelineStats {
            total_events: total.0,
            events_today: today.0,
            events_this_week: this_week.0,
            events_this_month: this_month.0,
            by_type: vec![],
            most_active_users: vec![],
            most_active_documents: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // ---- compute_streak_from_days tests ----

    #[test]
    fn test_streak_empty_days_returns_zero() {
        let today = d(2026, 2, 19);
        assert_eq!(compute_streak_from_days(&[], today), 0);
    }

    #[test]
    fn test_streak_single_day_today() {
        let today = d(2026, 2, 19);
        let days = vec![today];
        assert_eq!(compute_streak_from_days(&days, today), 1);
    }

    #[test]
    fn test_streak_single_day_yesterday() {
        let today = d(2026, 2, 19);
        let yesterday = today - Duration::days(1);
        let days = vec![yesterday];
        assert_eq!(compute_streak_from_days(&days, today), 1);
    }

    #[test]
    fn test_streak_single_day_two_days_ago_returns_zero() {
        let today = d(2026, 2, 19);
        let two_days_ago = today - Duration::days(2);
        let days = vec![two_days_ago];
        assert_eq!(compute_streak_from_days(&days, today), 0);
    }

    #[test]
    fn test_streak_consecutive_5_days() {
        let today = d(2026, 2, 19);
        let days: Vec<NaiveDate> = (0..5).map(|i| today - Duration::days(i)).collect();
        assert_eq!(compute_streak_from_days(&days, today), 5);
    }

    #[test]
    fn test_streak_break_in_sequence() {
        let today = d(2026, 2, 19);
        // Today, yesterday, then a gap (2026-02-16, skipping 2026-02-17)
        let days = vec![
            today,
            today - Duration::days(1),
            today - Duration::days(3), // gap
            today - Duration::days(4),
        ];
        assert_eq!(compute_streak_from_days(&days, today), 2);
    }

    #[test]
    fn test_streak_starting_yesterday_consecutive() {
        let today = d(2026, 2, 19);
        let days: Vec<NaiveDate> = (1..=4).map(|i| today - Duration::days(i)).collect();
        assert_eq!(compute_streak_from_days(&days, today), 4);
    }

    #[test]
    fn test_streak_ten_consecutive_days() {
        let today = d(2026, 2, 19);
        let days: Vec<NaiveDate> = (0..10).map(|i| today - Duration::days(i)).collect();
        assert_eq!(compute_streak_from_days(&days, today), 10);
    }

    // ---- compute_heatmap_level tests ----

    #[test]
    fn test_heatmap_level_max_value_zero_returns_zero() {
        assert_eq!(compute_heatmap_level(0, 0), 0);
    }

    #[test]
    fn test_heatmap_level_equals_max_returns_4() {
        assert_eq!(compute_heatmap_level(10, 10), 4);
    }

    #[test]
    fn test_heatmap_level_quarter_returns_1() {
        // 2.5/10 = 0.25 * 4 = 1.0.ceil() = 1
        assert_eq!(compute_heatmap_level(25, 100), 1);
    }

    #[test]
    fn test_heatmap_level_half_returns_2() {
        // 50/100 = 0.5 * 4 = 2.0.ceil() = 2
        assert_eq!(compute_heatmap_level(50, 100), 2);
    }

    #[test]
    fn test_heatmap_level_three_quarters_returns_3() {
        // 75/100 = 0.75 * 4 = 3.0.ceil() = 3
        assert_eq!(compute_heatmap_level(75, 100), 3);
    }

    #[test]
    fn test_heatmap_level_one_of_hundred_returns_1() {
        // 1/100 = 0.01 * 4 = 0.04.ceil() = 1
        assert_eq!(compute_heatmap_level(1, 100), 1);
    }

    #[test]
    fn test_heatmap_level_never_exceeds_4() {
        // Very large value with small max should cap at 4
        assert_eq!(compute_heatmap_level(1000, 1), 4);
    }
}

