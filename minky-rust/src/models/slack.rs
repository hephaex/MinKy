use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Supported messaging platforms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessagingPlatform {
    Slack,
    Teams,
    Discord,
}

impl std::fmt::Display for MessagingPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessagingPlatform::Slack => write!(f, "slack"),
            MessagingPlatform::Teams => write!(f, "teams"),
            MessagingPlatform::Discord => write!(f, "discord"),
        }
    }
}

/// Platform connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub id: i32,
    pub platform: MessagingPlatform,
    pub workspace_id: String,
    pub workspace_name: String,
    pub bot_token: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a platform connection
#[derive(Debug, Deserialize)]
pub struct CreatePlatformConfig {
    pub platform: MessagingPlatform,
    pub workspace_id: String,
    pub workspace_name: String,
    pub bot_token: String,
}

/// A message captured from a messaging platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMessage {
    pub id: String,
    pub platform: MessagingPlatform,
    pub workspace_id: String,
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub user_id: String,
    pub username: Option<String>,
    pub text: String,
    pub thread_ts: Option<String>,
    pub reply_count: i32,
    pub reactions: Vec<MessageReaction>,
    pub attachments: Vec<MessageAttachment>,
    pub posted_at: DateTime<Utc>,
    pub captured_at: DateTime<Utc>,
}

/// Emoji reaction on a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReaction {
    pub emoji: String,
    pub count: i32,
    pub users: Vec<String>,
}

/// File attachment in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttachment {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub url: Option<String>,
}

/// A conversation thread (series of related messages)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub thread_ts: String,
    pub platform: MessagingPlatform,
    pub workspace_id: String,
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub participant_count: i32,
    pub message_count: i32,
    pub messages: Vec<PlatformMessage>,
    pub started_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
}

/// Knowledge extraction result from a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedKnowledge {
    pub conversation_id: String,
    pub platform: MessagingPlatform,

    /// Short title for the extracted knowledge
    pub title: String,

    /// Summary of the knowledge captured
    pub summary: String,

    /// Key insights identified in the conversation
    pub insights: Vec<String>,

    /// Problem that was solved (if any)
    pub problem_solved: Option<String>,

    /// Technologies or tools mentioned
    pub technologies: Vec<String>,

    /// Who should know about this knowledge (role labels)
    pub relevant_for: Vec<String>,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,

    /// Whether this knowledge has been confirmed by a human
    pub confirmed: bool,

    pub extracted_at: DateTime<Utc>,
}

impl ExtractedKnowledge {
    /// Determine the minimum confidence to consider extraction useful
    pub fn is_high_quality(&self) -> bool {
        self.confidence >= 0.7
    }

    /// Produce a markdown-formatted summary for storage
    pub fn to_markdown(&self) -> String {
        let mut md = format!("# {}\n\n", self.title);
        md.push_str(&format!("**Source**: {} conversation\n\n", self.platform));
        md.push_str(&format!("## Summary\n{}\n\n", self.summary));

        if let Some(ref problem) = self.problem_solved {
            md.push_str(&format!("## Problem Solved\n{}\n\n", problem));
        }

        if !self.insights.is_empty() {
            md.push_str("## Key Insights\n");
            for insight in &self.insights {
                md.push_str(&format!("- {}\n", insight));
            }
            md.push('\n');
        }

        if !self.technologies.is_empty() {
            md.push_str(&format!(
                "## Technologies\n{}\n\n",
                self.technologies.join(", ")
            ));
        }

        if !self.relevant_for.is_empty() {
            md.push_str(&format!(
                "## Relevant For\n{}\n\n",
                self.relevant_for.join(", ")
            ));
        }

        md.push_str(&format!(
            "_Extracted at: {} (confidence: {:.0}%)_\n",
            self.extracted_at.format("%Y-%m-%d %H:%M UTC"),
            self.confidence * 100.0,
        ));

        md
    }
}

/// Status of a knowledge extraction job
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionStatus {
    #[default]
    Pending,
    Processing,
    Completed,
    Failed,
    Skipped,
}

impl std::fmt::Display for ExtractionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractionStatus::Pending => write!(f, "pending"),
            ExtractionStatus::Processing => write!(f, "processing"),
            ExtractionStatus::Completed => write!(f, "completed"),
            ExtractionStatus::Failed => write!(f, "failed"),
            ExtractionStatus::Skipped => write!(f, "skipped"),
        }
    }
}

/// Summary of extraction activity
#[derive(Debug, Serialize)]
pub struct ExtractionSummary {
    pub total_conversations: i64,
    pub total_messages: i64,
    pub knowledge_items_extracted: i64,
    pub high_quality_items: i64,
    pub pending_confirmation: i64,
    pub last_extraction_at: Option<DateTime<Utc>>,
}

/// Filter criteria for querying captured messages
#[derive(Debug, Default, Deserialize)]
pub struct MessageFilter {
    pub platform: Option<MessagingPlatform>,
    pub channel_id: Option<String>,
    pub user_id: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
}

impl MessageFilter {
    /// Clamp limit to a sensible maximum
    pub fn effective_limit(&self) -> i32 {
        self.limit.unwrap_or(50).min(200)
    }
}

/// Request to confirm or reject extracted knowledge
#[derive(Debug, Deserialize)]
pub struct ConfirmKnowledgeRequest {
    pub extraction_id: String,
    pub confirmed: bool,
    pub title_override: Option<String>,
    pub summary_override: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_knowledge() -> ExtractedKnowledge {
        ExtractedKnowledge {
            conversation_id: "C123/1700000000.000".to_string(),
            platform: MessagingPlatform::Slack,
            title: "How to set up pgvector".to_string(),
            summary: "Team discussed pgvector installation steps.".to_string(),
            insights: vec!["Use apt for Ubuntu".to_string()],
            problem_solved: Some("pgvector install failure".to_string()),
            technologies: vec!["PostgreSQL".to_string(), "pgvector".to_string()],
            relevant_for: vec!["backend-engineer".to_string()],
            confidence: 0.85,
            confirmed: false,
            extracted_at: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
        }
    }

    #[test]
    fn test_messaging_platform_display_slack() {
        assert_eq!(MessagingPlatform::Slack.to_string(), "slack");
    }

    #[test]
    fn test_messaging_platform_display_teams() {
        assert_eq!(MessagingPlatform::Teams.to_string(), "teams");
    }

    #[test]
    fn test_messaging_platform_display_discord() {
        assert_eq!(MessagingPlatform::Discord.to_string(), "discord");
    }

    #[test]
    fn test_messaging_platform_serde_roundtrip() {
        let platforms = [
            MessagingPlatform::Slack,
            MessagingPlatform::Teams,
            MessagingPlatform::Discord,
        ];
        for p in &platforms {
            let json = serde_json::to_string(p).unwrap();
            let back: MessagingPlatform = serde_json::from_str(&json).unwrap();
            assert_eq!(p, &back);
        }
    }

    #[test]
    fn test_extraction_status_default_is_pending() {
        assert_eq!(ExtractionStatus::default(), ExtractionStatus::Pending);
    }

    #[test]
    fn test_extraction_status_display() {
        assert_eq!(ExtractionStatus::Pending.to_string(), "pending");
        assert_eq!(ExtractionStatus::Processing.to_string(), "processing");
        assert_eq!(ExtractionStatus::Completed.to_string(), "completed");
        assert_eq!(ExtractionStatus::Failed.to_string(), "failed");
        assert_eq!(ExtractionStatus::Skipped.to_string(), "skipped");
    }

    #[test]
    fn test_extraction_status_serde_roundtrip() {
        let statuses = [
            ExtractionStatus::Pending,
            ExtractionStatus::Processing,
            ExtractionStatus::Completed,
            ExtractionStatus::Failed,
            ExtractionStatus::Skipped,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let back: ExtractionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(s, &back);
        }
    }

    #[test]
    fn test_extracted_knowledge_high_quality_at_threshold() {
        let mut k = sample_knowledge();
        k.confidence = 0.7;
        assert!(k.is_high_quality());
    }

    #[test]
    fn test_extracted_knowledge_low_quality_below_threshold() {
        let mut k = sample_knowledge();
        k.confidence = 0.69;
        assert!(!k.is_high_quality());
    }

    #[test]
    fn test_extracted_knowledge_to_markdown_contains_title() {
        let k = sample_knowledge();
        let md = k.to_markdown();
        assert!(md.contains("# How to set up pgvector"));
    }

    #[test]
    fn test_extracted_knowledge_to_markdown_contains_platform() {
        let k = sample_knowledge();
        let md = k.to_markdown();
        assert!(md.contains("slack"));
    }

    #[test]
    fn test_extracted_knowledge_to_markdown_contains_problem_solved() {
        let k = sample_knowledge();
        let md = k.to_markdown();
        assert!(md.contains("pgvector install failure"));
    }

    #[test]
    fn test_extracted_knowledge_to_markdown_no_problem_section_when_none() {
        let mut k = sample_knowledge();
        k.problem_solved = None;
        let md = k.to_markdown();
        assert!(!md.contains("## Problem Solved"));
    }

    #[test]
    fn test_extracted_knowledge_to_markdown_contains_technologies() {
        let k = sample_knowledge();
        let md = k.to_markdown();
        assert!(md.contains("PostgreSQL"));
        assert!(md.contains("pgvector"));
    }

    #[test]
    fn test_extracted_knowledge_to_markdown_contains_insights() {
        let k = sample_knowledge();
        let md = k.to_markdown();
        assert!(md.contains("Use apt for Ubuntu"));
    }

    #[test]
    fn test_extracted_knowledge_to_markdown_contains_confidence() {
        let k = sample_knowledge();
        let md = k.to_markdown();
        assert!(md.contains("85%"));
    }

    #[test]
    fn test_message_filter_default_limit_is_fifty() {
        let filter = MessageFilter::default();
        assert_eq!(filter.effective_limit(), 50);
    }

    #[test]
    fn test_message_filter_clamps_to_two_hundred() {
        let filter = MessageFilter {
            limit: Some(999),
            ..Default::default()
        };
        assert_eq!(filter.effective_limit(), 200);
    }

    #[test]
    fn test_message_filter_respects_small_limit() {
        let filter = MessageFilter {
            limit: Some(10),
            ..Default::default()
        };
        assert_eq!(filter.effective_limit(), 10);
    }
}
