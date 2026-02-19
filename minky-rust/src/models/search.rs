use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Search query parameters
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub category_id: Option<i32>,
    pub tags: Option<Vec<String>>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub sort_by: Option<SortField>,
    pub sort_order: Option<SortOrder>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SortField {
    #[default]
    Relevance,
    CreatedAt,
    UpdatedAt,
    Title,
    ViewCount,
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}


/// Search result item
#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub id: Uuid,
    pub title: String,
    pub content_snippet: String,
    pub highlights: Vec<String>,
    pub score: f32,
    pub category_name: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Search response with pagination
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub total: i64,
    pub page: i32,
    pub limit: i32,
    pub took_ms: u64,
    pub facets: SearchFacets,
}

/// Search facets for filtering
#[derive(Debug, Serialize)]
pub struct SearchFacets {
    pub categories: Vec<FacetCount>,
    pub tags: Vec<FacetCount>,
    pub date_ranges: Vec<DateRangeFacet>,
}

#[derive(Debug, Serialize)]
pub struct FacetCount {
    pub value: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct DateRangeFacet {
    pub label: String,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub count: i64,
}

/// OpenSearch document structure
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchDocument {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category_id: Option<i32>,
    pub category_name: Option<String>,
    pub tags: Vec<String>,
    pub user_id: i32,
    pub author_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub view_count: i32,
    pub embedding: Option<Vec<f32>>,
}

/// Korean text analysis result
#[derive(Debug, Serialize)]
pub struct KoreanAnalysis {
    pub tokens: Vec<KoreanToken>,
    pub nouns: Vec<String>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct KoreanToken {
    pub surface: String,
    pub pos: String,
    pub reading: Option<String>,
}

/// Autocomplete suggestion
#[derive(Debug, Serialize)]
pub struct AutocompleteSuggestion {
    pub text: String,
    pub score: f32,
    pub document_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_field_default_is_relevance() {
        assert!(matches!(SortField::default(), SortField::Relevance));
    }

    #[test]
    fn test_sort_order_default_is_desc() {
        assert!(matches!(SortOrder::default(), SortOrder::Desc));
    }

    #[test]
    fn test_sort_field_all_variants_deserialize() {
        let fields = [
            ("relevance", SortField::Relevance),
            ("created_at", SortField::CreatedAt),
            ("updated_at", SortField::UpdatedAt),
            ("title", SortField::Title),
            ("view_count", SortField::ViewCount),
        ];
        for (raw, expected) in fields {
            let val: SortField = serde_json::from_str(&format!("\"{}\"", raw)).unwrap();
            assert!(matches!(
                (&val, &expected),
                (SortField::Relevance, SortField::Relevance)
                    | (SortField::CreatedAt, SortField::CreatedAt)
                    | (SortField::UpdatedAt, SortField::UpdatedAt)
                    | (SortField::Title, SortField::Title)
                    | (SortField::ViewCount, SortField::ViewCount)
            ));
            let _ = val;
        }
    }

    #[test]
    fn test_sort_order_asc_deserializes() {
        let val: SortOrder = serde_json::from_str("\"asc\"").unwrap();
        assert!(matches!(val, SortOrder::Asc));
    }

    #[test]
    fn test_sort_order_desc_deserializes() {
        let val: SortOrder = serde_json::from_str("\"desc\"").unwrap();
        assert!(matches!(val, SortOrder::Desc));
    }

    #[test]
    fn test_search_hit_serializes() {
        use chrono::Utc;
        use uuid::Uuid;
        let hit = SearchHit {
            id: Uuid::new_v4(),
            title: "Test".to_string(),
            content_snippet: "snippet".to_string(),
            highlights: vec!["hi".to_string()],
            score: 0.95,
            category_name: Some("Tech".to_string()),
            tags: vec!["rust".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&hit).unwrap();
        assert!(json.contains("\"title\":\"Test\""));
        assert!(json.contains("\"score\":0.95"));
        assert!(json.contains("\"category_name\":\"Tech\""));
    }

    #[test]
    fn test_search_hit_no_category_serializes_null() {
        use chrono::Utc;
        use uuid::Uuid;
        let hit = SearchHit {
            id: Uuid::new_v4(),
            title: "No Cat".to_string(),
            content_snippet: "".to_string(),
            highlights: vec![],
            score: 0.5,
            category_name: None,
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&hit).unwrap();
        assert!(json.contains("\"category_name\":null"));
    }

    #[test]
    fn test_facet_count_serializes() {
        let facet = FacetCount {
            value: "Rust".to_string(),
            count: 42,
        };
        let json = serde_json::to_string(&facet).unwrap();
        assert!(json.contains("\"value\":\"Rust\""));
        assert!(json.contains("\"count\":42"));
    }

    #[test]
    fn test_autocomplete_suggestion_serializes() {
        let s = AutocompleteSuggestion {
            text: "rust lang".to_string(),
            score: 0.87,
            document_count: 15,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"text\":\"rust lang\""));
        assert!(json.contains("\"document_count\":15"));
    }

    #[test]
    fn test_korean_analysis_serializes() {
        let analysis = KoreanAnalysis {
            tokens: vec![KoreanToken {
                surface: "러스트".to_string(),
                pos: "NNG".to_string(),
                reading: Some("러스트".to_string()),
            }],
            nouns: vec!["러스트".to_string()],
            keywords: vec!["Rust".to_string()],
        };
        let json = serde_json::to_string(&analysis).unwrap();
        assert!(json.contains("\"surface\":\"러스트\""));
        assert!(json.contains("\"nouns\":[\"러스트\"]"));
    }

    #[test]
    fn test_korean_token_no_reading_serializes_null() {
        let token = KoreanToken {
            surface: "이다".to_string(),
            pos: "VCP".to_string(),
            reading: None,
        };
        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("\"reading\":null"));
    }

    #[test]
    fn test_search_document_with_embedding_serializes() {
        use chrono::Utc;
        let doc = SearchDocument {
            id: "doc-1".to_string(),
            title: "RAG Guide".to_string(),
            content: "content here".to_string(),
            category_id: Some(3),
            category_name: Some("AI".to_string()),
            tags: vec!["rag".to_string(), "llm".to_string()],
            user_id: 1,
            author_name: "Alice".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            view_count: 10,
            embedding: Some(vec![0.1, 0.2, 0.3]),
        };
        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("\"title\":\"RAG Guide\""));
        assert!(json.contains("\"embedding\":[0.1,0.2,0.3]"));
    }

    #[test]
    fn test_search_document_without_embedding_serializes_null() {
        use chrono::Utc;
        let doc = SearchDocument {
            id: "doc-2".to_string(),
            title: "No Embedding".to_string(),
            content: "content".to_string(),
            category_id: None,
            category_name: None,
            tags: vec![],
            user_id: 2,
            author_name: "Bob".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            view_count: 0,
            embedding: None,
        };
        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("\"embedding\":null"));
    }
}
