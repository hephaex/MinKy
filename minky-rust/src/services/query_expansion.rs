//! Query Expansion Service
//!
//! Uses Claude to generate alternative query phrasings for improved search recall.
//! Inspired by QMD's query expansion model approach.

use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    error::{AppError, AppResult},
    services::anthropic_types::{AnthropicMessage, AnthropicRequest, AnthropicResponse},
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Query expansion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExpansionRequest {
    /// Original query to expand
    pub query: String,
    /// Number of alternative queries to generate
    #[serde(default = "default_num_expansions")]
    pub num_expansions: usize,
    /// Domain context (optional)
    pub domain: Option<String>,
}

fn default_num_expansions() -> usize { 3 }

/// Query expansion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExpansionResponse {
    /// Original query
    pub original: String,
    /// Expanded alternative queries
    pub expansions: Vec<ExpandedQuery>,
    /// Total tokens used
    pub tokens_used: u32,
}

/// A single expanded query with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandedQuery {
    /// The expanded query text
    pub query: String,
    /// Type of expansion applied
    pub expansion_type: ExpansionType,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

/// Type of query expansion
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpansionType {
    /// Synonym replacement
    Synonym,
    /// Rephrasing
    Rephrase,
    /// More specific version
    Specific,
    /// More general version
    General,
    /// Related concept
    Related,
}

// ---------------------------------------------------------------------------
// Query Expansion Service
// ---------------------------------------------------------------------------

/// Service for expanding search queries using LLM
pub struct QueryExpansionService {
    config: Config,
    http_client: reqwest::Client,
}

impl QueryExpansionService {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Expand a query into multiple alternative phrasings
    pub async fn expand(&self, req: QueryExpansionRequest) -> AppResult<QueryExpansionResponse> {
        let api_key = self.config.anthropic_api_key.as_ref()
            .ok_or_else(|| AppError::Configuration("Anthropic API key not configured".into()))?;

        let system_prompt = r#"You are a search query expansion expert. Given a search query, generate alternative phrasings that would help find relevant documents.

Rules:
1. Generate exactly the requested number of alternative queries
2. Each alternative should capture different aspects or phrasings
3. Include synonyms, related concepts, and different specificity levels
4. Keep alternatives concise and search-friendly
5. Output ONLY the alternatives, one per line, no numbering or bullets"#;

        let domain_context = req.domain
            .map(|d| format!("\nDomain context: {d}"))
            .unwrap_or_default();

        let user_content = format!(
            "Generate {n} alternative search queries for:\n\n\"{query}\"{domain}",
            n = req.num_expansions,
            query = req.query,
            domain = domain_context
        );

        let request = AnthropicRequest {
            model: "claude-haiku-4-5-20251101".to_string(),
            max_tokens: 256,
            system: Some(system_prompt.to_string()),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: user_content,
            }],
        };

        let response = self.http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key.expose_secret())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("API request failed: {e}")))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!("API error: {error_text}")));
        }

        let result: AnthropicResponse = response.json().await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse response: {e}")))?;

        let text = result.content.first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        let expansions = parse_expansions(&text, &req.query);

        let tokens_used = result.usage
            .map(|u| u.input_tokens + u.output_tokens)
            .unwrap_or(0);

        Ok(QueryExpansionResponse {
            original: req.query,
            expansions,
            tokens_used,
        })
    }

    /// Fast local expansion without LLM (for hybrid/fallback)
    pub fn expand_local(&self, query: &str) -> Vec<ExpandedQuery> {
        let mut expansions = Vec::new();

        // Synonym expansions
        let synonym_map = [
            ("error", vec!["issue", "problem", "bug", "failure"]),
            ("fix", vec!["solve", "resolve", "repair", "correct"]),
            ("create", vec!["make", "build", "generate", "add"]),
            ("delete", vec!["remove", "drop", "clear", "erase"]),
            ("update", vec!["modify", "change", "edit", "revise"]),
            ("find", vec!["search", "locate", "discover", "get"]),
            ("how to", vec!["guide", "tutorial", "steps to", "way to"]),
            ("what is", vec!["definition", "meaning of", "explain"]),
        ];

        let lower = query.to_lowercase();

        for (term, synonyms) in synonym_map {
            if lower.contains(term) {
                for syn in synonyms.iter().take(2) {
                    let expanded = lower.replace(term, syn);
                    if expanded != lower {
                        expansions.push(ExpandedQuery {
                            query: expanded,
                            expansion_type: ExpansionType::Synonym,
                            confidence: 0.8,
                        });
                    }
                }
            }
        }

        // Add more general version (drop qualifiers)
        let general = remove_qualifiers(&lower);
        if general != lower && !general.is_empty() {
            expansions.push(ExpandedQuery {
                query: general,
                expansion_type: ExpansionType::General,
                confidence: 0.6,
            });
        }

        // Limit to 3 expansions
        expansions.truncate(3);
        expansions
    }
}

// ---------------------------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------------------------

/// Parse LLM response into structured expansions
fn parse_expansions(text: &str, original: &str) -> Vec<ExpandedQuery> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| line.trim().to_lowercase() != original.to_lowercase())
        .map(|line| {
            let cleaned = line.trim()
                .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == '-' || c == '*')
                .trim();

            let expansion_type = classify_expansion(original, cleaned);

            ExpandedQuery {
                query: cleaned.to_string(),
                expansion_type,
                confidence: 0.85,
            }
        })
        .collect()
}

/// Classify the type of expansion based on original and expanded query
fn classify_expansion(original: &str, expanded: &str) -> ExpansionType {
    let orig_words: Vec<&str> = original.split_whitespace().collect();
    let exp_words: Vec<&str> = expanded.split_whitespace().collect();

    // More words = more specific
    if exp_words.len() > orig_words.len() + 1 {
        return ExpansionType::Specific;
    }

    // Fewer words = more general
    if exp_words.len() < orig_words.len() - 1 {
        return ExpansionType::General;
    }

    // Similar length, check for synonyms
    let shared = orig_words.iter()
        .filter(|w| exp_words.contains(w))
        .count();

    // At least half the words are shared = synonym replacement
    if shared >= orig_words.len() / 2 && shared > 0 {
        ExpansionType::Synonym
    } else if shared > 0 {
        ExpansionType::Rephrase
    } else {
        ExpansionType::Related
    }
}

/// Remove common qualifiers to create a more general query
fn remove_qualifiers(query: &str) -> String {
    let qualifiers = [
        "very", "extremely", "really", "specific", "exact",
        "best", "top", "latest", "new", "old",
        "quickly", "easily", "simply", "just",
    ];

    let result: Vec<&str> = query.split_whitespace()
        .filter(|word| !qualifiers.contains(&word.to_lowercase().as_str()))
        .collect();

    // Keep at least 2 words
    if result.len() < 2 {
        return query.to_string();
    }

    result.join(" ")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    fn test_config() -> Config {
        Config {
            environment: "test".to_string(),
            host: "127.0.0.1".to_string(),
            port: 3000,
            database_url: "postgres://test:test@localhost/test".to_string(),
            database_max_connections: 5,
            database_min_connections: 1,
            database_acquire_timeout_secs: 30,
            database_max_lifetime_secs: 0,
            database_idle_timeout_secs: 0,
            jwt_secret: SecretString::from("test-secret-key-for-testing-only"),
            jwt_expiration_hours: 24,
            opensearch_url: None,
            openai_api_key: None,
            anthropic_api_key: None,
            git_repo_path: None,
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_signing_secret: None,
            cors_allowed_origins: "http://localhost:3000".to_string(),
        }
    }

    #[test]
    fn test_parse_expansions_basic() {
        let text = "how to resolve issue\nways to fix bug\nerror resolution guide";
        let expansions = parse_expansions(text, "how to fix error");

        assert_eq!(expansions.len(), 3);
        assert_eq!(expansions[0].query, "how to resolve issue");
    }

    #[test]
    fn test_parse_expansions_filters_original() {
        let text = "how to fix error\nalternative query";
        let expansions = parse_expansions(text, "how to fix error");

        assert_eq!(expansions.len(), 1);
        assert_eq!(expansions[0].query, "alternative query");
    }

    #[test]
    fn test_parse_expansions_cleans_numbering() {
        let text = "1. first query\n2. second query\n- third query";
        let expansions = parse_expansions(text, "original");

        assert_eq!(expansions[0].query, "first query");
        assert_eq!(expansions[1].query, "second query");
        assert_eq!(expansions[2].query, "third query");
    }

    #[test]
    fn test_classify_expansion_specific() {
        let typ = classify_expansion("fix error", "fix critical authentication error");
        assert_eq!(typ, ExpansionType::Specific);
    }

    #[test]
    fn test_classify_expansion_general() {
        let typ = classify_expansion("fix critical authentication error", "fix error");
        assert_eq!(typ, ExpansionType::General);
    }

    #[test]
    fn test_classify_expansion_synonym() {
        let typ = classify_expansion("fix error", "resolve error");
        assert_eq!(typ, ExpansionType::Synonym);
    }

    #[test]
    fn test_remove_qualifiers() {
        let result = remove_qualifiers("very specific best error fix");
        assert_eq!(result, "error fix");
    }

    #[test]
    fn test_remove_qualifiers_preserves_minimum() {
        let result = remove_qualifiers("very");
        assert_eq!(result, "very");
    }

    #[test]
    fn test_local_expand_synonyms() {
        let service = QueryExpansionService::new(test_config());
        let expansions = service.expand_local("how to fix error");

        assert!(!expansions.is_empty());
        let queries: Vec<&str> = expansions.iter().map(|e| e.query.as_str()).collect();
        // Should contain synonym replacements
        assert!(queries.iter().any(|q| q.contains("solve") || q.contains("issue")));
    }

    #[test]
    fn test_local_expand_empty_for_no_matches() {
        let service = QueryExpansionService::new(test_config());
        let expansions = service.expand_local("xyzzy foobar");

        // May have general expansion but likely empty for unknown terms
        assert!(expansions.len() <= 1);
    }

    #[test]
    fn test_expansion_type_serde() {
        let json = serde_json::to_string(&ExpansionType::Synonym).unwrap();
        assert_eq!(json, "\"synonym\"");

        let parsed: ExpansionType = serde_json::from_str("\"rephrase\"").unwrap();
        assert_eq!(parsed, ExpansionType::Rephrase);
    }
}
