use crate::models::{
    ExtractedKnowledge, ExtractionStatus, MessageFilter, MessagingPlatform, PlatformMessage,
};

/// Pure helpers for Slack/Teams knowledge extraction pipeline.
///
/// Network-bound operations (API calls, DB writes) live in the route handlers
/// so that all business logic here is unit-testable without mocks.
pub struct SlackService;

impl SlackService {
    pub fn new() -> Self {
        Self
    }

    /// Decide whether a message thread is worth analysing for knowledge.
    ///
    /// Criteria:
    /// - At least `min_replies` reply messages in the thread
    /// - Thread has at least 2 distinct participants
    /// - Total character count of all messages is above `min_chars`
    pub fn is_thread_worth_analysing(
        messages: &[PlatformMessage],
        min_replies: usize,
        min_chars: usize,
    ) -> bool {
        if messages.len() < min_replies + 1 {
            return false;
        }

        let total_chars: usize = messages.iter().map(|m| m.text.len()).sum();
        if total_chars < min_chars {
            return false;
        }

        let unique_users: std::collections::HashSet<&str> =
            messages.iter().map(|m| m.user_id.as_str()).collect();

        unique_users.len() >= 2
    }

    /// Build a single text block from a slice of messages suitable for
    /// sending to an LLM for knowledge extraction.
    ///
    /// Format:
    /// ```text
    /// [username]: message text
    /// [username]: reply text
    /// ```
    pub fn build_conversation_prompt(messages: &[PlatformMessage]) -> String {
        messages
            .iter()
            .map(|m| {
                let name = m.username.as_deref().unwrap_or(&m.user_id);
                format!("[{}]: {}", name, m.text)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Parse the JSON response returned by the LLM into `ExtractedKnowledge`.
    ///
    /// Returns `None` if the JSON is malformed or fields are missing.
    pub fn parse_extraction_response(
        conversation_id: &str,
        platform: MessagingPlatform,
        raw: &str,
    ) -> Option<ExtractedKnowledge> {
        // Strip markdown code fences if present
        let cleaned = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let value: serde_json::Value = serde_json::from_str(cleaned).ok()?;

        let title = value["title"].as_str()?.to_string();
        let summary = value["summary"].as_str()?.to_string();

        let insights = value["insights"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let problem_solved = value["problem_solved"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let technologies = value["technologies"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let relevant_for = value["relevant_for"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let confidence = value["confidence"].as_f64().unwrap_or(0.5) as f32;

        Some(ExtractedKnowledge {
            conversation_id: conversation_id.to_string(),
            platform,
            title,
            summary,
            insights,
            problem_solved,
            technologies,
            relevant_for,
            confidence: confidence.clamp(0.0, 1.0),
            confirmed: false,
            extracted_at: chrono::Utc::now(),
        })
    }

    /// Filter a list of messages according to the provided criteria.
    pub fn apply_filter<'a>(
        messages: &'a [PlatformMessage],
        filter: &MessageFilter,
    ) -> Vec<&'a PlatformMessage> {
        let limit = filter.effective_limit() as usize;

        messages
            .iter()
            .filter(|m| {
                if let Some(ref platform) = filter.platform {
                    if &m.platform != platform {
                        return false;
                    }
                }
                if let Some(ref channel_id) = filter.channel_id {
                    if &m.channel_id != channel_id {
                        return false;
                    }
                }
                if let Some(ref user_id) = filter.user_id {
                    if &m.user_id != user_id {
                        return false;
                    }
                }
                if let Some(since) = filter.since {
                    if m.posted_at < since {
                        return false;
                    }
                }
                true
            })
            .take(limit)
            .collect()
    }

    /// Determine extraction status from extracted knowledge properties.
    pub fn classify_status(knowledge: &ExtractedKnowledge) -> ExtractionStatus {
        if knowledge.title.is_empty() || knowledge.summary.is_empty() {
            return ExtractionStatus::Failed;
        }
        if knowledge.confidence < 0.3 {
            return ExtractionStatus::Skipped;
        }
        if knowledge.confirmed {
            ExtractionStatus::Completed
        } else {
            ExtractionStatus::Pending
        }
    }
}

impl Default for SlackService {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about conversations in a channel
#[derive(Debug)]
pub struct ConversationStats {
    pub total_messages: usize,
    pub unique_users: usize,
    pub thread_count: usize,
    pub avg_thread_length: f32,
}

impl ConversationStats {
    /// Compute stats from a flat list of messages (grouped by thread_ts).
    pub fn compute(messages: &[PlatformMessage]) -> Self {
        let total_messages = messages.len();

        let unique_users: std::collections::HashSet<&str> =
            messages.iter().map(|m| m.user_id.as_str()).collect();

        // Group into threads by thread_ts; root messages have thread_ts == None
        let mut threads: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();

        for msg in messages {
            let key = msg
                .thread_ts
                .as_deref()
                .unwrap_or(msg.id.as_str());
            *threads.entry(key).or_insert(0) += 1;
        }

        let thread_count = threads.len();
        let avg_thread_length = if thread_count == 0 {
            0.0
        } else {
            total_messages as f32 / thread_count as f32
        };

        ConversationStats {
            total_messages,
            unique_users: unique_users.len(),
            thread_count,
            avg_thread_length,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::models::{MessageAttachment, MessageReaction};
    use chrono::Utc;

    fn make_message(id: &str, user_id: &str, text: &str, thread_ts: Option<&str>) -> PlatformMessage {
        PlatformMessage {
            id: id.to_string(),
            platform: MessagingPlatform::Slack,
            workspace_id: "W1".to_string(),
            channel_id: "C1".to_string(),
            channel_name: Some("general".to_string()),
            user_id: user_id.to_string(),
            username: Some(user_id.to_string()),
            text: text.to_string(),
            thread_ts: thread_ts.map(str::to_string),
            reply_count: 0,
            reactions: vec![],
            attachments: vec![],
            posted_at: Utc::now(),
            captured_at: Utc::now(),
        }
    }

    // is_thread_worth_analysing tests

    #[test]
    fn test_worth_analysing_empty_messages_returns_false() {
        assert!(!SlackService::is_thread_worth_analysing(&[], 1, 10));
    }

    #[test]
    fn test_worth_analysing_single_user_returns_false() {
        let messages = vec![
            make_message("1", "alice", "hello there", None),
            make_message("2", "alice", "any updates?", None),
        ];
        assert!(!SlackService::is_thread_worth_analysing(&messages, 1, 5));
    }

    #[test]
    fn test_worth_analysing_two_users_sufficient_replies() {
        let messages = vec![
            make_message("1", "alice", "how do we install pgvector?", Some("ts1")),
            make_message("2", "bob", "you can use apt-get install postgresql-pgvector", Some("ts1")),
        ];
        assert!(SlackService::is_thread_worth_analysing(&messages, 1, 20));
    }

    #[test]
    fn test_worth_analysing_too_few_chars_returns_false() {
        let messages = vec![
            make_message("1", "alice", "hi", Some("ts1")),
            make_message("2", "bob", "ok", Some("ts1")),
        ];
        // total chars = 4, min_chars = 100
        assert!(!SlackService::is_thread_worth_analysing(&messages, 1, 100));
    }

    // build_conversation_prompt tests

    #[test]
    fn test_build_prompt_empty_returns_empty_string() {
        assert_eq!(SlackService::build_conversation_prompt(&[]), "");
    }

    #[test]
    fn test_build_prompt_uses_username_when_available() {
        let messages = vec![make_message("1", "U1", "hello world", None)];
        let prompt = SlackService::build_conversation_prompt(&messages);
        assert!(prompt.contains("[U1]: hello world"));
    }

    #[test]
    fn test_build_prompt_falls_back_to_user_id_when_no_username() {
        let mut msg = make_message("1", "U_UNKNOWN", "test", None);
        msg.username = None;
        let prompt = SlackService::build_conversation_prompt(&[msg]);
        assert!(prompt.contains("[U_UNKNOWN]: test"));
    }

    #[test]
    fn test_build_prompt_multiple_messages_joined_by_newline() {
        let messages = vec![
            make_message("1", "alice", "first", None),
            make_message("2", "bob", "second", None),
        ];
        let prompt = SlackService::build_conversation_prompt(&messages);
        assert_eq!(prompt, "[alice]: first\n[bob]: second");
    }

    // parse_extraction_response tests

    #[test]
    fn test_parse_response_valid_json() {
        let raw = r#"{
            "title": "pgvector setup",
            "summary": "How to install pgvector",
            "insights": ["use apt"],
            "problem_solved": "install error",
            "technologies": ["PostgreSQL"],
            "relevant_for": ["backend"],
            "confidence": 0.9
        }"#;

        let result = SlackService::parse_extraction_response(
            "C1/ts1",
            MessagingPlatform::Slack,
            raw,
        );
        assert!(result.is_some());
        let k = result.unwrap();
        assert_eq!(k.title, "pgvector setup");
        assert_eq!(k.confidence, 0.9);
    }

    #[test]
    fn test_parse_response_strips_markdown_fences() {
        let raw = "```json\n{\"title\":\"T\",\"summary\":\"S\",\"confidence\":0.8}\n```";
        let result = SlackService::parse_extraction_response(
            "C1/ts1",
            MessagingPlatform::Teams,
            raw,
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_response_invalid_json_returns_none() {
        let result = SlackService::parse_extraction_response(
            "C1/ts1",
            MessagingPlatform::Slack,
            "not json at all",
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_response_missing_title_returns_none() {
        let raw = r#"{"summary":"S","confidence":0.8}"#;
        let result = SlackService::parse_extraction_response(
            "C1/ts1",
            MessagingPlatform::Slack,
            raw,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_response_confidence_clamped_to_one() {
        let raw = r#"{"title":"T","summary":"S","confidence":2.5}"#;
        let result = SlackService::parse_extraction_response(
            "C1/ts1",
            MessagingPlatform::Slack,
            raw,
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().confidence, 1.0);
    }

    #[test]
    fn test_parse_response_empty_problem_solved_maps_to_none() {
        let raw = r#"{"title":"T","summary":"S","problem_solved":"","confidence":0.8}"#;
        let result = SlackService::parse_extraction_response(
            "C1/ts1",
            MessagingPlatform::Slack,
            raw,
        );
        assert!(result.is_some());
        assert!(result.unwrap().problem_solved.is_none());
    }

    // classify_status tests

    #[test]
    fn test_classify_status_empty_title_is_failed() {
        let mut k = sample_knowledge();
        k.title = String::new();
        assert_eq!(SlackService::classify_status(&k), ExtractionStatus::Failed);
    }

    #[test]
    fn test_classify_status_low_confidence_is_skipped() {
        let mut k = sample_knowledge();
        k.confidence = 0.2;
        assert_eq!(SlackService::classify_status(&k), ExtractionStatus::Skipped);
    }

    #[test]
    fn test_classify_status_confirmed_is_completed() {
        let mut k = sample_knowledge();
        k.confirmed = true;
        assert_eq!(SlackService::classify_status(&k), ExtractionStatus::Completed);
    }

    #[test]
    fn test_classify_status_unconfirmed_good_quality_is_pending() {
        let k = sample_knowledge();
        assert_eq!(SlackService::classify_status(&k), ExtractionStatus::Pending);
    }

    // apply_filter tests

    #[test]
    fn test_apply_filter_by_channel() {
        let messages = vec![
            {
                let mut m = make_message("1", "alice", "msg", None);
                m.channel_id = "C_A".to_string();
                m
            },
            {
                let mut m = make_message("2", "bob", "msg", None);
                m.channel_id = "C_B".to_string();
                m
            },
        ];
        let filter = MessageFilter {
            channel_id: Some("C_A".to_string()),
            ..Default::default()
        };
        let result = SlackService::apply_filter(&messages, &filter);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].channel_id, "C_A");
    }

    #[test]
    fn test_apply_filter_respects_limit() {
        let messages: Vec<_> = (0..10)
            .map(|i| make_message(&i.to_string(), "alice", "x", None))
            .collect();
        let filter = MessageFilter {
            limit: Some(3),
            ..Default::default()
        };
        let result = SlackService::apply_filter(&messages, &filter);
        assert_eq!(result.len(), 3);
    }

    // ConversationStats tests

    #[test]
    fn test_stats_empty_messages() {
        let stats = ConversationStats::compute(&[]);
        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.unique_users, 0);
        assert_eq!(stats.thread_count, 0);
        assert_eq!(stats.avg_thread_length, 0.0);
    }

    #[test]
    fn test_stats_single_thread_two_messages() {
        let messages = vec![
            make_message("1", "alice", "q", Some("ts1")),
            make_message("2", "bob", "a", Some("ts1")),
        ];
        let stats = ConversationStats::compute(&messages);
        assert_eq!(stats.total_messages, 2);
        assert_eq!(stats.unique_users, 2);
        assert_eq!(stats.thread_count, 1);
        assert!((stats.avg_thread_length - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_stats_two_threads() {
        let messages = vec![
            make_message("1", "alice", "q", Some("ts1")),
            make_message("2", "bob", "a", Some("ts1")),
            make_message("3", "carol", "other q", Some("ts2")),
        ];
        let stats = ConversationStats::compute(&messages);
        assert_eq!(stats.thread_count, 2);
    }

    fn sample_knowledge() -> ExtractedKnowledge {
        ExtractedKnowledge {
            conversation_id: "C1/ts1".to_string(),
            platform: MessagingPlatform::Slack,
            title: "Title".to_string(),
            summary: "Summary".to_string(),
            insights: vec![],
            problem_solved: None,
            technologies: vec![],
            relevant_for: vec![],
            confidence: 0.8,
            confirmed: false,
            extracted_at: chrono::Utc::now(),
        }
    }

    // Additional is_thread_worth_analysing edge cases

    #[test]
    fn test_worth_analysing_exactly_min_replies_passes() {
        // min_replies = 1, so we need at least 2 messages (1 root + 1 reply)
        let messages = vec![
            make_message("1", "alice", &"a".repeat(60), Some("ts1")),
            make_message("2", "bob", &"b".repeat(60), Some("ts1")),
        ];
        assert!(SlackService::is_thread_worth_analysing(&messages, 1, 50));
    }

    #[test]
    fn test_worth_analysing_exactly_min_chars_passes() {
        // 3 chars per message * 2 messages = 6, min_chars = 6
        let messages = vec![
            make_message("1", "alice", "abc", Some("ts1")),
            make_message("2", "bob", "def", Some("ts1")),
        ];
        assert!(SlackService::is_thread_worth_analysing(&messages, 1, 6));
    }

    #[test]
    fn test_worth_analysing_one_less_than_min_chars_fails() {
        let messages = vec![
            make_message("1", "alice", "abc", Some("ts1")),
            make_message("2", "bob", "de", Some("ts1")),
        ];
        // total = 5, min_chars = 6
        assert!(!SlackService::is_thread_worth_analysing(&messages, 1, 6));
    }

    #[test]
    fn test_worth_analysing_three_distinct_users_passes() {
        let messages = vec![
            make_message("1", "alice", &"x".repeat(50), Some("ts1")),
            make_message("2", "bob",   &"y".repeat(50), Some("ts1")),
            make_message("3", "carol", &"z".repeat(50), Some("ts1")),
        ];
        assert!(SlackService::is_thread_worth_analysing(&messages, 2, 50));
    }

    // Additional build_conversation_prompt tests

    #[test]
    fn test_build_prompt_single_message() {
        let msg = make_message("1", "alice", "hello world", None);
        let prompt = SlackService::build_conversation_prompt(&[msg]);
        assert_eq!(prompt, "[alice]: hello world");
    }

    #[test]
    fn test_build_prompt_preserves_message_order() {
        let messages = vec![
            make_message("1", "a", "first", None),
            make_message("2", "b", "second", None),
            make_message("3", "c", "third", None),
        ];
        let prompt = SlackService::build_conversation_prompt(&messages);
        let lines: Vec<&str> = prompt.split('\n').collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("first"));
        assert!(lines[1].contains("second"));
        assert!(lines[2].contains("third"));
    }

    // Additional classify_status tests

    #[test]
    fn test_classify_status_empty_summary_is_failed() {
        let mut k = sample_knowledge();
        k.summary = String::new();
        assert_eq!(SlackService::classify_status(&k), ExtractionStatus::Failed);
    }

    #[test]
    fn test_classify_status_exactly_threshold_confidence_is_pending() {
        let mut k = sample_knowledge();
        k.confidence = 0.3; // exactly at skip boundary
        // 0.3 is NOT below 0.3, so it should be Pending
        assert_eq!(SlackService::classify_status(&k), ExtractionStatus::Pending);
    }

    #[test]
    fn test_classify_status_just_below_threshold_is_skipped() {
        let mut k = sample_knowledge();
        k.confidence = 0.29;
        assert_eq!(SlackService::classify_status(&k), ExtractionStatus::Skipped);
    }

    // Additional apply_filter tests

    #[test]
    fn test_apply_filter_no_filter_returns_all() {
        let messages: Vec<_> = (0..5)
            .map(|i| make_message(&i.to_string(), "alice", "text", None))
            .collect();
        let filter = MessageFilter::default();
        let result = SlackService::apply_filter(&messages, &filter);
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_apply_filter_by_user_id() {
        let messages = vec![
            make_message("1", "U_alice", "hi", None),
            make_message("2", "U_bob", "hello", None),
            make_message("3", "U_alice", "bye", None),
        ];
        let filter = MessageFilter {
            user_id: Some("U_alice".to_string()),
            ..Default::default()
        };
        let result = SlackService::apply_filter(&messages, &filter);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|m| m.user_id == "U_alice"));
    }

    #[test]
    fn test_apply_filter_by_platform() {
        let mut slack_msg = make_message("1", "alice", "msg", None);
        slack_msg.platform = MessagingPlatform::Slack;
        let mut teams_msg = make_message("2", "bob", "msg", None);
        teams_msg.platform = MessagingPlatform::Teams;

        let filter = MessageFilter {
            platform: Some(MessagingPlatform::Slack),
            ..Default::default()
        };
        let all_messages = [slack_msg, teams_msg];
        let result = SlackService::apply_filter(&all_messages, &filter);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].platform, MessagingPlatform::Slack);
    }

    // Additional ConversationStats tests

    #[test]
    fn test_stats_no_thread_ts_each_message_is_own_thread() {
        let messages = vec![
            make_message("1", "alice", "q", None),
            make_message("2", "bob", "a", None),
        ];
        let stats = ConversationStats::compute(&messages);
        // Each message without thread_ts is its own "thread" keyed by id
        assert_eq!(stats.thread_count, 2);
    }

    #[test]
    fn test_stats_avg_length_calculated_correctly() {
        let messages = vec![
            make_message("1", "alice", "q", Some("ts1")),
            make_message("2", "bob", "a", Some("ts1")),
            make_message("3", "carol", "q2", Some("ts2")),
            make_message("4", "dave", "a2", Some("ts2")),
        ];
        let stats = ConversationStats::compute(&messages);
        assert_eq!(stats.thread_count, 2);
        assert!((stats.avg_thread_length - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_stats_unique_users_counted_correctly() {
        let messages = vec![
            make_message("1", "alice", "a", Some("ts1")),
            make_message("2", "alice", "b", Some("ts1")),
            make_message("3", "bob", "c", Some("ts1")),
        ];
        let stats = ConversationStats::compute(&messages);
        assert_eq!(stats.unique_users, 2);
    }
}
