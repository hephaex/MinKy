use anyhow::Result;
use opensearch::{
    http::{
        request::JsonBody,
        response::Response,
        transport::{SingleNodeConnectionPool, TransportBuilder},
    },
    indices::{IndicesCreateParts, IndicesExistsParts},
    BulkParts, DeleteParts, IndexParts, OpenSearch, SearchParts,
};
use serde_json::{json, Value};
use std::time::Instant;

use crate::{
    config::Config,
    error::{AppError, AppResult},
    models::{
        AutocompleteSuggestion, FacetCount, SearchDocument, SearchFacets, SearchHit, SearchQuery,
        SearchResponse, SortField, SortOrder,
    },
};

const INDEX_NAME: &str = "minky_documents";

/// OpenSearch service for document search
pub struct SearchService {
    client: OpenSearch,
}

impl SearchService {
    pub async fn new(config: &Config) -> Result<Self> {
        let url = config.opensearch_url.as_ref()
            .ok_or_else(|| anyhow::anyhow!("OpenSearch URL not configured"))?;

        let url = opensearch::http::Url::parse(url)?;
        let pool = SingleNodeConnectionPool::new(url);
        let transport = TransportBuilder::new(pool).build()?;
        let client = OpenSearch::new(transport);

        Ok(Self { client })
    }

    /// Create search index with mappings
    pub async fn create_index(&self) -> Result<()> {
        let exists = self.client
            .indices()
            .exists(IndicesExistsParts::Index(&[INDEX_NAME]))
            .send()
            .await?;

        if exists.status_code().is_success() {
            return Ok(());
        }

        let mapping = json!({
            "settings": {
                "number_of_shards": 1,
                "number_of_replicas": 0,
                "analysis": {
                    "analyzer": {
                        "korean": {
                            "type": "custom",
                            "tokenizer": "nori_tokenizer",
                            "filter": ["lowercase", "nori_part_of_speech"]
                        }
                    }
                }
            },
            "mappings": {
                "properties": {
                    "id": { "type": "keyword" },
                    "title": {
                        "type": "text",
                        "analyzer": "korean",
                        "fields": {
                            "keyword": { "type": "keyword" },
                            "suggest": { "type": "completion" }
                        }
                    },
                    "content": {
                        "type": "text",
                        "analyzer": "korean"
                    },
                    "category_id": { "type": "integer" },
                    "category_name": { "type": "keyword" },
                    "tags": { "type": "keyword" },
                    "user_id": { "type": "integer" },
                    "author_name": { "type": "keyword" },
                    "created_at": { "type": "date" },
                    "updated_at": { "type": "date" },
                    "view_count": { "type": "integer" },
                    "embedding": {
                        "type": "knn_vector",
                        "dimension": 1536,
                        "method": {
                            "name": "hnsw",
                            "space_type": "cosinesimil",
                            "engine": "nmslib"
                        }
                    }
                }
            }
        });

        self.client
            .indices()
            .create(IndicesCreateParts::Index(INDEX_NAME))
            .body(mapping)
            .send()
            .await?;

        Ok(())
    }

    /// Index a document
    pub async fn index_document(&self, doc: SearchDocument) -> Result<()> {
        let doc_id = doc.id.clone();
        self.client
            .index(IndexParts::IndexId(INDEX_NAME, &doc_id))
            .body(doc)
            .send()
            .await?;

        Ok(())
    }

    /// Delete a document from index
    pub async fn delete_document(&self, id: &str) -> Result<()> {
        self.client
            .delete(DeleteParts::IndexId(INDEX_NAME, id))
            .send()
            .await?;

        Ok(())
    }

    /// Bulk index documents
    pub async fn bulk_index(&self, documents: Vec<SearchDocument>) -> Result<usize> {
        let mut body: Vec<JsonBody<Value>> = Vec::new();

        for doc in &documents {
            body.push(json!({ "index": { "_index": INDEX_NAME, "_id": &doc.id } }).into());
            body.push(serde_json::to_value(doc)?.into());
        }

        let response: Response = self.client
            .bulk(BulkParts::None)
            .body(body)
            .send()
            .await?;

        let result: Value = response.json().await?;
        let errors = result["errors"].as_bool().unwrap_or(false);

        if errors {
            tracing::warn!("Bulk indexing had some errors");
        }

        Ok(documents.len())
    }

    /// Search documents
    pub async fn search(&self, query: SearchQuery) -> AppResult<SearchResponse> {
        let start = Instant::now();

        let page = query.page.unwrap_or(1).max(1);
        let limit = query.limit.unwrap_or(20).min(100);
        let from = (page - 1) * limit;

        // Build query
        let mut must_clauses = vec![
            json!({
                "multi_match": {
                    "query": query.q,
                    "fields": ["title^3", "content", "tags^2"],
                    "type": "best_fields",
                    "fuzziness": "AUTO"
                }
            })
        ];

        // Add filters
        if let Some(category_id) = query.category_id {
            must_clauses.push(json!({
                "term": { "category_id": category_id }
            }));
        }

        if let Some(tags) = &query.tags {
            if !tags.is_empty() {
                must_clauses.push(json!({
                    "terms": { "tags": tags }
                }));
            }
        }

        // Date range filter
        if query.date_from.is_some() || query.date_to.is_some() {
            let mut range = json!({});
            if let Some(from) = query.date_from {
                range["gte"] = json!(from.to_rfc3339());
            }
            if let Some(to) = query.date_to {
                range["lte"] = json!(to.to_rfc3339());
            }
            must_clauses.push(json!({
                "range": { "created_at": range }
            }));
        }

        // Build sort
        let sort = match query.sort_by.unwrap_or_default() {
            SortField::Relevance => json!([{ "_score": "desc" }]),
            SortField::CreatedAt => {
                let order = match query.sort_order.unwrap_or_default() {
                    SortOrder::Asc => "asc",
                    SortOrder::Desc => "desc",
                };
                json!([{ "created_at": order }])
            }
            SortField::UpdatedAt => {
                let order = match query.sort_order.unwrap_or_default() {
                    SortOrder::Asc => "asc",
                    SortOrder::Desc => "desc",
                };
                json!([{ "updated_at": order }])
            }
            SortField::Title => {
                let order = match query.sort_order.unwrap_or_default() {
                    SortOrder::Asc => "asc",
                    SortOrder::Desc => "desc",
                };
                json!([{ "title.keyword": order }])
            }
            SortField::ViewCount => {
                let order = match query.sort_order.unwrap_or_default() {
                    SortOrder::Asc => "asc",
                    SortOrder::Desc => "desc",
                };
                json!([{ "view_count": order }])
            }
        };

        let search_body = json!({
            "query": {
                "bool": {
                    "must": must_clauses
                }
            },
            "sort": sort,
            "from": from,
            "size": limit,
            "highlight": {
                "fields": {
                    "title": {},
                    "content": { "fragment_size": 150, "number_of_fragments": 3 }
                }
            },
            "aggs": {
                "categories": {
                    "terms": { "field": "category_name", "size": 20 }
                },
                "tags": {
                    "terms": { "field": "tags", "size": 50 }
                }
            }
        });

        let response = self.client
            .search(SearchParts::Index(&[INDEX_NAME]))
            .body(search_body)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Search failed: {}", e)))?;

        let result: Value = response.json().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse search response: {}", e)))?;

        let total = result["hits"]["total"]["value"].as_i64().unwrap_or(0);

        let hits: Vec<SearchHit> = result["hits"]["hits"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|hit| {
                let source = &hit["_source"];
                let highlight = &hit["highlight"];

                let highlights: Vec<String> = highlight["content"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|h| h.as_str().map(String::from))
                    .collect();

                SearchHit {
                    id: uuid::Uuid::parse_str(source["id"].as_str().unwrap_or("")).unwrap_or_default(),
                    title: source["title"].as_str().unwrap_or("").to_string(),
                    content_snippet: highlights.first().cloned().unwrap_or_default(),
                    highlights,
                    score: hit["_score"].as_f64().unwrap_or(0.0) as f32,
                    category_name: source["category_name"].as_str().map(String::from),
                    tags: source["tags"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|t| t.as_str().map(String::from))
                        .collect(),
                    created_at: chrono::DateTime::parse_from_rfc3339(
                        source["created_at"].as_str().unwrap_or("")
                    ).map(|dt| dt.with_timezone(&chrono::Utc)).unwrap_or_default(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(
                        source["updated_at"].as_str().unwrap_or("")
                    ).map(|dt| dt.with_timezone(&chrono::Utc)).unwrap_or_default(),
                }
            })
            .collect();

        // Parse facets
        let category_facets: Vec<FacetCount> = result["aggregations"]["categories"]["buckets"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|b| FacetCount {
                value: b["key"].as_str().unwrap_or("").to_string(),
                count: b["doc_count"].as_i64().unwrap_or(0),
            })
            .collect();

        let tag_facets: Vec<FacetCount> = result["aggregations"]["tags"]["buckets"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|b| FacetCount {
                value: b["key"].as_str().unwrap_or("").to_string(),
                count: b["doc_count"].as_i64().unwrap_or(0),
            })
            .collect();

        let took_ms = start.elapsed().as_millis() as u64;

        Ok(SearchResponse {
            hits,
            total,
            page,
            limit,
            took_ms,
            facets: SearchFacets {
                categories: category_facets,
                tags: tag_facets,
                date_ranges: vec![],
            },
        })
    }

    /// Semantic search using embeddings
    pub async fn semantic_search(&self, embedding: Vec<f32>, limit: i32) -> AppResult<Vec<SearchHit>> {
        let search_body = json!({
            "size": limit,
            "query": {
                "knn": {
                    "embedding": {
                        "vector": embedding,
                        "k": limit
                    }
                }
            }
        });

        let response = self.client
            .search(SearchParts::Index(&[INDEX_NAME]))
            .body(search_body)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Semantic search failed: {}", e)))?;

        let result: Value = response.json().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse response: {}", e)))?;

        let hits: Vec<SearchHit> = result["hits"]["hits"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|hit| {
                let source = &hit["_source"];
                SearchHit {
                    id: uuid::Uuid::parse_str(source["id"].as_str().unwrap_or("")).unwrap_or_default(),
                    title: source["title"].as_str().unwrap_or("").to_string(),
                    content_snippet: source["content"].as_str().unwrap_or("").chars().take(200).collect(),
                    highlights: vec![],
                    score: hit["_score"].as_f64().unwrap_or(0.0) as f32,
                    category_name: source["category_name"].as_str().map(String::from),
                    tags: source["tags"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|t| t.as_str().map(String::from))
                        .collect(),
                    created_at: chrono::DateTime::parse_from_rfc3339(
                        source["created_at"].as_str().unwrap_or("")
                    ).map(|dt| dt.with_timezone(&chrono::Utc)).unwrap_or_default(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(
                        source["updated_at"].as_str().unwrap_or("")
                    ).map(|dt| dt.with_timezone(&chrono::Utc)).unwrap_or_default(),
                }
            })
            .collect();

        Ok(hits)
    }

    /// Autocomplete suggestions
    pub async fn autocomplete(&self, prefix: &str, limit: i32) -> Result<Vec<AutocompleteSuggestion>> {
        let search_body = json!({
            "suggest": {
                "title-suggest": {
                    "prefix": prefix,
                    "completion": {
                        "field": "title.suggest",
                        "size": limit,
                        "skip_duplicates": true
                    }
                }
            }
        });

        let response = self.client
            .search(SearchParts::Index(&[INDEX_NAME]))
            .body(search_body)
            .send()
            .await?;

        let result: Value = response.json().await?;

        let suggestions: Vec<AutocompleteSuggestion> = result["suggest"]["title-suggest"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|s| s["options"].as_array())
            .unwrap_or(&vec![])
            .iter()
            .map(|opt| AutocompleteSuggestion {
                text: opt["text"].as_str().unwrap_or("").to_string(),
                score: opt["_score"].as_f64().unwrap_or(0.0) as f32,
                document_count: 1,
            })
            .collect();

        Ok(suggestions)
    }
}
