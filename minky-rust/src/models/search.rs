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
}
