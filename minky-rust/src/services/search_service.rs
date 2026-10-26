use anyhow::Result;
use opensearch::{
    http::{
        transport::{SingleNodeConnectionPool, TransportBuilder},
    },
    indices::{IndicesCreateParts},
    DeleteParts, IndexParts, OpenSearch, SearchParts,
};
use serde_json::{json, Value};
use std::collections::HashSet;
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
    /// Raw OpenSearch URL for direct HTTP operations (bypasses opensearch-rs for bulk)
    opensearch_url: String,
    http: reqwest::Client,
    /// Shared mecab-ko tokenizer used to pre-tokenize BOTH indexed documents
    /// (see `bulk_index`/`index_document` callers) and incoming search
    /// queries, keeping the two sides symmetric against the whitespace-only
    /// OpenSearch analyzer (see `create_index`). Without tokenizing the
    /// query side too, un-segmented Korean queries would never match the
    /// segmented morpheme index. `None` falls back to sending the raw query
    /// text, matching the whitespace-heuristic fallback used elsewhere when
    /// no dictionary is configured.
    korean_tokenizer: Option<crate::services::SharedKoreanTokenizer>,
}

impl SearchService {
    pub async fn new(config: &Config) -> Result<Self> {
        Self::with_korean_tokenizer(config, None).await
    }

    /// Create a `SearchService` that also pre-tokenizes search queries with
    /// the given shared mecab-ko tokenizer, matching the pre-tokenization
    /// applied to indexed documents.
    pub async fn with_korean_tokenizer(
        config: &Config,
        korean_tokenizer: Option<crate::services::SharedKoreanTokenizer>,
    ) -> Result<Self> {
        let url = config
            .opensearch_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("OpenSearch URL not configured"))?;

        let os_url = opensearch::http::Url::parse(url)?;
        let pool = SingleNodeConnectionPool::new(os_url);
        let transport = TransportBuilder::new(pool).build()?;
        let client = OpenSearch::new(transport);

        Ok(Self {
            client,
            opensearch_url: url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
            korean_tokenizer,
        })
    }

    /// Pre-tokenize `text` for OpenSearch (query or document side) using the
    /// same mecab-ko morpheme-surface + whitespace-join logic as
    /// `KoreanService::tokenize_for_lexical_index`, so both sides of the
    /// whitespace-analyzed index stay symmetric. Falls back to returning
    /// `text` unchanged when no tokenizer is configured, matching the
    /// pre-existing (pre-mecab-ko) behavior for that case.
    fn tokenize_for_search(&self, text: &str) -> String {
        crate::services::tokenize_for_lexical_index_with(self.korean_tokenizer.as_ref(), text)
    }

    /// Create search index with mappings
    /// Create search index with mappings if it does not already exist.
    ///
    /// Uses PUT (create) which returns 400 `resource_already_exists_exception`
    /// when the index exists — that error is silently swallowed.  We avoid
    /// HEAD `indices.exists()` because opensearch-rs 2.3 attempts to decode
    /// the empty HEAD response body and panics with an EOF parse error against
    /// OpenSearch 3.x.
    pub async fn create_index(&self) -> Result<()> {
        let mapping = json!({
            "settings": {
                "number_of_shards": 1,
                "number_of_replicas": 0,
                "analysis": {
                    "analyzer": {
                        // Text is pre-tokenized on the Rust side (mecab-ko
                        // morphological analysis, see
                        // korean_service::KoreanService::tokenize) into
                        // whitespace-joined morphemes before being sent to
                        // OpenSearch, so lexical indexing only needs a plain
                        // whitespace+lowercase analyzer here. This replaces
                        // the previous nori_tokenizer/nori_part_of_speech
                        // OpenSearch-side Korean analyzer (mechanism B2 per
                        // the C2 plan: one shared tokenizer pipeline reused
                        // for both lexical indexing and pre-embedding
                        // preprocessing, instead of duplicating Korean NLP
                        // logic in both Rust and an OpenSearch plugin).
                        "korean": {
                            "type": "custom",
                            "tokenizer": "whitespace",
                            "filter": ["lowercase"]
                        },
                        "cjk_bigram_korean": {
                            "type": "custom",
                            "tokenizer": "whitespace",
                            "filter": ["lowercase", "cjk_bigram"]
                        },
                        "ngram_2": {
                            "type": "custom",
                            "tokenizer": "ngram_tokenizer",
                            "filter": ["lowercase"]
                        }
                    },
                    "tokenizer": {
                        "ngram_tokenizer": {
                            "type": "ngram",
                            "min_gram": 2,
                            "max_gram": 2,
                            "token_chars": ["letter", "digit"]
                        }
                    }
                }
            },
            "mappings": {
                "properties": {
                    "id": { "type": "keyword" },
                    "title": {
                        "type": "text",
                        "fields": {
                            "keyword": { "type": "keyword" },
                            "suggest": { "type": "completion" }
                        }
                    },
                    "content": {
                        "type": "text"
                    },
                    // Pre-tokenized (mecab-ko morpheme, whitespace-joined)
                    // copies used for lexical matching/highlighting against
                    // the whitespace-only "korean" analyzer above. "title"/
                    // "content" above stay as the original readable text
                    // (default analyzer) so _source retrieval never returns
                    // corrupted/tokenized display text.
                    //
                    // The cjk_bigram/ngram multi-field subfields add
                    // 2-character windows over the same text so a query can
                    // still match a substring of a compound morpheme (e.g.
                    // "쿠버" inside "쿠버네티스") when no whole morpheme
                    // token matches.
                    "title_tokenized": {
                        "type": "text",
                        "analyzer": "korean",
                        "fields": {
                            "cjk_bigram": {
                                "type": "text",
                                "analyzer": "cjk_bigram_korean"
                            },
                            "ngram": {
                                "type": "text",
                                "analyzer": "ngram_2"
                            }
                        }
                    },
                    "content_tokenized": {
                        "type": "text",
                        "analyzer": "korean",
                        "fields": {
                            "cjk_bigram": {
                                "type": "text",
                                "analyzer": "cjk_bigram_korean"
                            },
                            "ngram": {
                                "type": "text",
                                "analyzer": "ngram_2"
                            }
                        }
                    },
                    "category_id": { "type": "integer" },
                    "category_name": { "type": "keyword" },
                    "tags": { "type": "keyword" },
                    "user_id": { "type": "integer" },
                    "author_name": { "type": "keyword" },
                    "created_at": { "type": "date" },
                    "updated_at": { "type": "date" },
                    "view_count": { "type": "integer" },
                    "sensitivity": { "type": "keyword" },
                    "is_public": { "type": "boolean" }
                    // NOTE: the previous "embedding": {"type": "knn_vector",
                    // "dimension": 1536, ...} field is intentionally removed.
                    // It was confirmed dead code: OpenSearch's knn_vector /
                    // dense-vector path was never populated or queried —
                    // Postgres/pgvector (see hybrid_search.rs) is the sole
                    // vector-search source of truth, and 1536 was already
                    // stale against the real 4096-dim embedding models
                    // (see EmbeddingModel::dimension). Re-adding this field
                    // without updating the dimension would silently break
                    // against real vectors on first use.
                }
            }
        });

        let response = self
            .client
            .indices()
            .create(IndicesCreateParts::Index(INDEX_NAME))
            .body(mapping)
            .send()
            .await?;

        let status = response.status_code();
        if status.is_success() {
            return Ok(());
        }

        // 400 resource_already_exists_exception = index exists, silently ignore
        if status == 400 {
            let body: Value = response.json().await.unwrap_or_default();
            let reason = body
                .pointer("/error/type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if reason == "resource_already_exists_exception" {
                return Ok(());
            }
            return Err(anyhow::anyhow!("Index creation failed: {}", body));
        }

        Err(anyhow::anyhow!(
            "Index creation returned unexpected status: {}",
            status
        ))
    }

    /// Delete the search index if it exists. Used by the operator-triggered
    /// full-reindex path to force a real mapping rebuild — `create_index`
    /// alone silently no-ops when the index already exists (see its 400
    /// `resource_already_exists_exception` handling), which is correct for
    /// normal startup but wrong for an explicit reindex request. A missing
    /// index (404) is treated as success since the end state (no index) is
    /// what the caller wants either way.
    pub async fn delete_index(&self) -> Result<()> {
        let url = format!("{}/{}", self.opensearch_url, INDEX_NAME);
        let response = self
            .http
            .delete(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Index delete request failed: {}", e))?;
        let status = response.status();
        if status.is_success() || status.as_u16() == 404 {
            return Ok(());
        }
        let body = response.text().await.unwrap_or_default();
        Err(anyhow::anyhow!(
            "Index deletion returned unexpected status {}: {}",
            status,
            body
        ))
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

    /// Bulk index documents using raw reqwest to work around opensearch-rs 2.3
    /// response-body decode incompatibility with OpenSearch 3.x.
    /// Sends in batches of 200 documents to keep individual requests small.
    pub async fn bulk_index(&self, documents: Vec<SearchDocument>) -> Result<usize> {
        if documents.is_empty() {
            return Ok(0);
        }

        const BATCH_SIZE: usize = 200;
        let url = format!("{}/_bulk", self.opensearch_url);
        let mut total = 0usize;

        for chunk in documents.chunks(BATCH_SIZE) {
            let mut ndjson = String::new();
            for doc in chunk {
                let action = json!({ "index": { "_index": INDEX_NAME, "_id": &doc.id } });
                ndjson.push_str(&action.to_string());
                ndjson.push('\n');
                ndjson.push_str(&serde_json::to_string(doc)?);
                ndjson.push('\n');
            }

            let resp = self
                .http
                .post(&url)
                .header("Content-Type", "application/x-ndjson")
                .body(ndjson)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("Bulk HTTP request failed: {}", e))?;

            let text = resp.text().await.unwrap_or_default();
            let errors = serde_json::from_str::<Value>(&text)
                .unwrap_or_default()
                .get("errors")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if errors {
                tracing::warn!("Bulk batch had errors: {}", &text[..text.len().min(300)]);
            }
            total += chunk.len();
        }

        tracing::debug!("Bulk complete: {} docs indexed", total);
        Ok(total)
    }



    /// Search documents.
    ///
    /// `user_id`: `None` means unauthenticated — confidential documents are excluded.
    pub async fn search(
        &self,
        query: SearchQuery,
        user_id: Option<i32>,
    ) -> AppResult<SearchResponse> {
        let start = Instant::now();

        let page = query.page.unwrap_or(1).clamp(1, 10_000);
        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        let from = (page - 1).saturating_mul(limit);

        // Build query. `title_tokenized`/`content_tokenized` hold the
        // mecab-ko-segmented, whitespace-joined copy of the readable
        // `title`/`content` text and are the only fields analyzed by the
        // whitespace `korean` analyzer (see create_index); `title`/`content`
        // themselves keep the default analyzer and are used only for
        // display, never for matching, so the query text must be tokenized
        // the same way here and matched against the *_tokenized fields, or
        // an un-segmented Korean query would never match the segmented
        // morpheme index. `tags` is a `keyword`-typed field (exact match),
        // so it always searches against the original raw query text.
        let tokenized_query = self.tokenize_for_search(&query.q);

        // Subfields (see create_index) are unboosted: whole-morpheme matches
        // on the main *_tokenized fields naturally outrank 2-char-window
        // subfield hits, and the subfields only decide a match when no whole
        // morpheme token matches (the compound-substring miss case, e.g.
        // 쿠버 inside 쿠버네티스). AUTO fuzziness is intentionally disabled
        // for Korean queries — fuzzy matching mangles non-Latin text.
        let mut must_clauses = vec![json!({
            "bool": {
                "should": [
                    {
                        "multi_match": {
                            "query": tokenized_query,
                            "fields": [
                                "title_tokenized^3",
                                "content_tokenized",
                                "title_tokenized.cjk_bigram",
                                "title_tokenized.ngram",
                                "content_tokenized.cjk_bigram",
                                "content_tokenized.ngram"
                            ],
                            "type": "best_fields"
                        }
                    },
                    {
                        "multi_match": {
                            "query": query.q,
                            "fields": ["tags^2"],
                            "type": "best_fields"
                        }
                    }
                ],
                "minimum_should_match": 1
            }
        })];

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

        // Unauthenticated callers cannot see confidential documents.
        let mut search_query = json!({
            "bool": {
                "must": must_clauses
            }
        });
        if user_id.is_none() {
            search_query["bool"]["must_not"] = json!([
                { "term": { "sensitivity": "confidential" } }
            ]);
        }

        let search_body = json!({
            "query": search_query,
            "sort": sort,
            "from": from,
            "size": limit,
            "highlight": {
                // Highlight against the ORIGINAL readable "title"/"content"
                // fields (default analyzer), not "*_tokenized" — otherwise
                // every Korean result would show morpheme-segmented,
                // space-mangled display text (a display-level recurrence of
                // the same _source-corruption class of bug fixed above).
                // `require_field_match: false` lets highlighting run
                // independently of which field the main query matched
                // against (title_tokenized/content_tokenized), and
                // `highlight_query` re-runs matching with the ORIGINAL raw
                // query text against the readable fields so highlighted
                // fragments are natural, un-tokenized text.
                "require_field_match": false,
                "fields": {
                    "title": {
                        "highlight_query": {
                            "match": { "title": { "query": query.q } }
                        }
                    },
                    "content": {
                        "fragment_size": 150,
                        "number_of_fragments": 3,
                        "highlight_query": {
                            "match": { "content": { "query": query.q } }
                        }
                    }
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

        let response = self
            .client
            .search(SearchParts::Index(&[INDEX_NAME]))
            .body(search_body)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Search failed: {}", e)))?;

        let result: Value = {
            let text = response.text().await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to read search response: {}", e))
            })?;
            serde_json::from_str(&text).map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to parse search response: {}", e))
            })?
        };

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
                    id: uuid::Uuid::parse_str(source["id"].as_str().unwrap_or(""))
                        .unwrap_or_default(),
                    title: source["title"].as_str().unwrap_or("").to_string(),
                    content_snippet: highlights.first().cloned().unwrap_or_else(|| {
                        truncate_content(source["content"].as_str().unwrap_or(""), 200)
                    }),
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
                        source["created_at"].as_str().unwrap_or(""),
                    )
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_default(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(
                        source["updated_at"].as_str().unwrap_or(""),
                    )
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_default(),
                }
            })
            .collect();

        // Same-title documents can dominate a query's top results (e.g.
        // reindexed copies). Deduplicate client-side, keeping the
        // highest-scored hit per title. NOTE: `total` intentionally stays
        // the raw matching count — pagination offsets still operate on the
        // full result set, so duplicate titles may reappear on later pages.
        let hits = dedupe_same_title(hits);

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

    /// Semantic search using embeddings.
    ///
    /// `user_id`: `None` means unauthenticated — confidential documents are excluded
    /// via a `post_filter` on the `sensitivity` field.
    pub async fn semantic_search(
        &self,
        embedding: Vec<f32>,
        limit: i32,
        user_id: Option<i32>,
    ) -> AppResult<Vec<SearchHit>> {
        let mut search_body = json!({
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

        if user_id.is_none() {
            search_body["post_filter"] = json!({
                "bool": {
                    "must_not": [
                        { "term": { "sensitivity": "confidential" } }
                    ]
                }
            });
        }

        let response = self
            .client
            .search(SearchParts::Index(&[INDEX_NAME]))
            .body(search_body)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Semantic search failed: {}", e)))?;

        let result: Value = response
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse response: {}", e)))?;

        let hits: Vec<SearchHit> = result["hits"]["hits"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|hit| {
                let source = &hit["_source"];
                SearchHit {
                    id: uuid::Uuid::parse_str(source["id"].as_str().unwrap_or(""))
                        .unwrap_or_default(),
                    title: source["title"].as_str().unwrap_or("").to_string(),
                    content_snippet: source["content"]
                        .as_str()
                        .unwrap_or("")
                        .chars()
                        .take(200)
                        .collect(),
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
                        source["created_at"].as_str().unwrap_or(""),
                    )
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_default(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(
                        source["updated_at"].as_str().unwrap_or(""),
                    )
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_default(),
                }
            })
            .collect();

        Ok(hits)
    }

    /// Autocomplete suggestions.
    ///
    /// `user_id`: `None` means unauthenticated — a `post_filter` restricts results
    /// so that confidential document titles are never surfaced.
    pub async fn autocomplete(
        &self,
        prefix: &str,
        limit: i32,
        user_id: Option<i32>,
    ) -> Result<Vec<AutocompleteSuggestion>> {
        let mut search_body = json!({
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

        if user_id.is_none() {
            search_body["post_filter"] = json!({
                "bool": {
                    "must_not": [
                        { "term": { "sensitivity": "confidential" } }
                    ]
                }
            });
        }

        let response = self
            .client
            .search(SearchParts::Index(&[INDEX_NAME]))
            .body(search_body)
            .send()
            .await?;

        let result: Value = {
            let text = response.text().await.unwrap_or_default();
            serde_json::from_str(&text).unwrap_or_default()
        };

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

/// Pure helper functions for search logic (testable without OpenSearch)
/// Clamp a page number to minimum 1
pub fn clamp_page(page: i64) -> i64 {
    page.max(1)
}

/// Clamp a limit to the range [1, 100]
pub fn clamp_limit(limit: i64) -> i64 {
    limit.clamp(1, 100)
}

/// Calculate `from` offset from page and limit (0-indexed)
pub fn calc_from(page: i64, limit: i64) -> i64 {
    (clamp_page(page) - 1) * clamp_limit(limit)
}

/// Determine the sort field string for OpenSearch based on SortField variant
pub fn sort_field_str(field: &SortField) -> &'static str {
    match field {
        SortField::Relevance => "_score",
        SortField::CreatedAt => "created_at",
        SortField::UpdatedAt => "updated_at",
        SortField::Title => "title.keyword",
        SortField::ViewCount => "view_count",
    }
}

/// Determine the sort order string for OpenSearch based on SortOrder variant
pub fn sort_order_str(order: &SortOrder) -> &'static str {
    match order {
        SortOrder::Asc => "asc",
        SortOrder::Desc => "desc",
    }
}

/// Extract text snippet from highlights (first entry, or empty)
pub fn first_highlight(highlights: &[String]) -> String {
    highlights.first().cloned().unwrap_or_default()
}

/// Truncate content to at most `max_chars` characters
pub fn truncate_content(content: &str, max_chars: usize) -> String {
    content.chars().take(max_chars).collect()
}

/// Return the `must_not` array that excludes confidential documents when the
/// caller is unauthenticated (`user_id == None`).  Returns `None` for
/// authenticated callers so the caller can skip adding the filter entirely.
/// Pure function — testable without OpenSearch.
pub fn build_confidential_filter(user_id: Option<i32>) -> Option<serde_json::Value> {
    if user_id.is_none() {
        Some(json!([{ "term": { "sensitivity": "confidential" } }]))
    } else {
        None
    }
}
/// Remove duplicate hits that share the same title, keeping the first
/// (highest-scored) occurrence. Hits are assumed to already be sorted by
/// score. Pure function — testable without OpenSearch.
pub fn dedupe_same_title(hits: Vec<SearchHit>) -> Vec<SearchHit> {
    let mut seen: HashSet<String> = HashSet::new();
    hits.into_iter()
        .filter(|hit| seen.insert(hit.title.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_page_zero_becomes_one() {
        assert_eq!(clamp_page(0), 1);
    }

    #[test]
    fn test_clamp_page_negative_becomes_one() {
        assert_eq!(clamp_page(-5), 1);
    }

    #[test]
    fn test_clamp_page_positive_unchanged() {
        assert_eq!(clamp_page(3), 3);
    }

    #[test]
    fn test_clamp_limit_over_max() {
        assert_eq!(clamp_limit(200), 100);
    }

    #[test]
    fn test_clamp_limit_zero_becomes_one() {
        assert_eq!(clamp_limit(0), 1);
    }

    #[test]
    fn test_clamp_limit_normal_unchanged() {
        assert_eq!(clamp_limit(20), 20);
    }

    #[test]
    fn test_calc_from_first_page() {
        assert_eq!(calc_from(1, 20), 0);
    }

    #[test]
    fn test_calc_from_second_page() {
        assert_eq!(calc_from(2, 20), 20);
    }

    #[test]
    fn test_calc_from_page_zero_treated_as_one() {
        assert_eq!(calc_from(0, 20), 0);
    }

    #[test]
    fn test_calc_from_large_page() {
        assert_eq!(calc_from(5, 10), 40);
    }

    #[test]
    fn test_sort_field_str_relevance() {
        assert_eq!(sort_field_str(&SortField::Relevance), "_score");
    }

    #[test]
    fn test_sort_field_str_created_at() {
        assert_eq!(sort_field_str(&SortField::CreatedAt), "created_at");
    }

    #[test]
    fn test_sort_field_str_updated_at() {
        assert_eq!(sort_field_str(&SortField::UpdatedAt), "updated_at");
    }

    #[test]
    fn test_sort_field_str_title() {
        assert_eq!(sort_field_str(&SortField::Title), "title.keyword");
    }

    #[test]
    fn test_sort_field_str_view_count() {
        assert_eq!(sort_field_str(&SortField::ViewCount), "view_count");
    }

    #[test]
    fn test_sort_order_str_asc() {
        assert_eq!(sort_order_str(&SortOrder::Asc), "asc");
    }

    #[test]
    fn test_sort_order_str_desc() {
        assert_eq!(sort_order_str(&SortOrder::Desc), "desc");
    }

    #[test]
    fn test_first_highlight_empty() {
        let highlights: Vec<String> = vec![];
        assert_eq!(first_highlight(&highlights), "");
    }

    #[test]
    fn test_first_highlight_returns_first() {
        let highlights = vec!["first".to_string(), "second".to_string()];
        assert_eq!(first_highlight(&highlights), "first");
    }

    #[test]
    fn test_truncate_content_shorter_than_max() {
        let s = "hello";
        assert_eq!(truncate_content(s, 100), "hello");
    }

    #[test]
    fn test_truncate_content_exact_max() {
        let s = "hello";
        assert_eq!(truncate_content(s, 5), "hello");
    }

    #[test]
    fn test_truncate_content_longer_than_max() {
        let s = "hello world";
        assert_eq!(truncate_content(s, 5), "hello");
    }

    #[test]
    fn test_truncate_content_unicode() {
        let s = "안녕하세요 세계";
        let truncated = truncate_content(s, 3);
        assert_eq!(truncated, "안녕하");
    }

    // -------------------------------------------------------------------------
    // Sensitivity filter logic tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_build_confidential_filter_unauthenticated_returns_some() {
        let filter = build_confidential_filter(None);
        assert!(filter.is_some());
        let v = filter.unwrap();
        let arr = v.as_array().expect("must_not should be an array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["term"]["sensitivity"], "confidential");
    }

    #[test]
    fn test_build_confidential_filter_authenticated_returns_none() {
        let filter = build_confidential_filter(Some(42));
        assert!(filter.is_none());
    }

    #[test]
    fn test_build_confidential_filter_user_id_zero_is_authenticated() {
        // user_id = 0 is still a valid authenticated user
        let filter = build_confidential_filter(Some(0));
        assert!(filter.is_none());
    }

    fn make_hit(id: uuid::Uuid, title: &str, score: f32) -> SearchHit {
        SearchHit {
            id,
            title: title.to_string(),
            content_snippet: String::new(),
            highlights: vec![],
            score,
            category_name: None,
            tags: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    // -------------------------------------------------------------------------
    // Same-title dedup tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_dedupe_same_title_keeps_first() {
        let id_a = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let id_b = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let hits = vec![
            make_hit(id_a, "중복 제목", 2.5),
            make_hit(id_b, "중복 제목", 1.0),
        ];
        let deduped = dedupe_same_title(hits);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].score, 2.5);
    }

    #[test]
    fn test_dedupe_same_title_distinct_titles_unchanged() {
        let id_a = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let id_b = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let hits = vec![
            make_hit(id_a, "제목 A", 3.0),
            make_hit(id_b, "제목 B", 2.0),
        ];
        let deduped = dedupe_same_title(hits);
        assert_eq!(deduped.len(), 2);
    }

    fn make_test_config() -> crate::config::Config {
        use secrecy::SecretString;
        crate::config::Config {
            environment: "test".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            database_url: "postgres://localhost/test".to_string(),
            database_max_connections: 5,
            database_min_connections: 1,
            database_acquire_timeout_secs: 30,
            database_max_lifetime_secs: 1800,
            database_idle_timeout_secs: 600,
            jwt_secret: SecretString::from("test-secret"),
            jwt_expiration_hours: 24,
            opensearch_url: Some("http://localhost:9200".to_string()),
            openai_api_key: None,
            upstage_api_key: None,
            git_repo_path: None,
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_signing_secret: None,
            cors_allowed_origins: "http://localhost:3000".to_string(),
            local_embedding_enabled: false,
            vault_watch: crate::config::VaultWatchConfig::default(),
        }
    }

    /// Regression test for the C2/mecab-ko OpenSearch mapping change.
    ///
    /// Verifies against a live OpenSearch instance that:
    /// 1. `create_index` succeeds without the removed `nori_tokenizer`
    ///    (proving nori is no longer a hard dependency of index creation).
    /// 2. The created index mapping has no `knn_vector` field (dead code
    ///    removed) and no analyzer referencing `nori_tokenizer` or
    ///    `nori_part_of_speech` (negative mapping assertion).
    /// 3. A document indexed with pre-tokenized (whitespace-joined morpheme)
    ///    title/content is retrievable via a lexical search on those terms,
    ///    proving the whitespace analyzer path works end-to-end.
    ///
    /// Requires a live OpenSearch at OPENSEARCH_URL (default
    /// http://localhost:9200). Skipped in CI; run with:
    ///   cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn create_index_has_no_nori_dependency_and_no_dead_knn_field() {
        let config = make_test_config();
        let service = SearchService::new(&config)
            .await
            .expect("failed to connect to OpenSearch");

        // Clean slate: delete the index if a prior run left it behind.
        let delete_url = format!("{}/minky_documents", config.opensearch_url.as_ref().unwrap());
        let _ = reqwest::Client::new().delete(&delete_url).send().await;

        service
            .create_index()
            .await
            .expect("create_index must succeed without nori_tokenizer installed/enabled");

        // Fetch the actual mapping back from OpenSearch and assert on it directly
        // (not just "no error"), proving the settings we sent are what's live.
        let mapping_url = format!(
            "{}/minky_documents/_mapping",
            config.opensearch_url.as_ref().unwrap()
        );
        let mapping_resp: serde_json::Value = reqwest::Client::new()
            .get(&mapping_url)
            .send()
            .await
            .expect("mapping fetch failed")
            .json()
            .await
            .expect("mapping response was not valid JSON");
        let mapping_str = mapping_resp.to_string();
        assert!(
            !mapping_str.contains("nori_tokenizer"),
            "mapping must not reference nori_tokenizer: {}",
            mapping_str
        );
        assert!(
            !mapping_str.contains("nori_part_of_speech"),
            "mapping must not reference nori_part_of_speech: {}",
            mapping_str
        );
        assert!(
            !mapping_str.contains("knn_vector"),
            "mapping must not contain the dead knn_vector field: {}",
            mapping_str
        );

        let settings_url = format!(
            "{}/minky_documents/_settings",
            config.opensearch_url.as_ref().unwrap()
        );
        let settings_resp: serde_json::Value = reqwest::Client::new()
            .get(&settings_url)
            .send()
            .await
            .expect("settings fetch failed")
            .json()
            .await
            .expect("settings response was not valid JSON");
        assert!(
            !settings_resp.to_string().contains("nori"),
            "index settings must not reference nori in any form"
        );

        // Write-path proof: run un-spaced Korean text through the real
        // KoreanService::tokenize_for_lexical_index preprocessing step
        // (mecab-ko morphological analysis, or the whitespace fallback if
        // MECAB_KO_DICDIR is unset) before indexing — mirroring exactly what
        // routes/search.rs::reindex_all does. If mecab-ko is not actually
        // invoked, "분석기입니다" stays a single unsplit token and would not
        // match a search for the substring "분석" as a whole word below.
        let korean_service =
            crate::services::KoreanService::with_tokenizer(sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap(), crate::services::korean_service::load_shared_korean_tokenizer());
        let raw_content = "한국어형태소분석기입니다";
        let tokenized_content = korean_service.tokenize_for_lexical_index(raw_content);
        assert_ne!(
            tokenized_content, raw_content,
            "tokenize_for_lexical_index must actually segment un-spaced Korean text (requires MECAB_KO_DICDIR to be set for this assertion to be meaningful)"
        );

        let doc = SearchDocument {
            id: "c2-mecab-ko-fixture".to_string(),
            title: "한국어 형태소 분석기".to_string(),
            content: raw_content.to_string(),
            title_tokenized: korean_service.tokenize_for_lexical_index("한국어 형태소 분석기"),
            content_tokenized: tokenized_content,
            category_id: None,
            category_name: None,
            tags: vec![],
            user_id: 1,
            author_name: "test".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            view_count: 0,
            embedding: None,
            sensitivity: "public".to_string(),
            is_public: true,
        };
        service
            .index_document(doc)
            .await
            .expect("index_document must succeed against the new mapping");

        // OpenSearch refresh so the doc is immediately searchable.
        let refresh_url = format!(
            "{}/minky_documents/_refresh",
            config.opensearch_url.as_ref().unwrap()
        );
        reqwest::Client::new()
            .post(&refresh_url)
            .send()
            .await
            .expect("refresh failed");

        let search_url = format!(
            "{}/minky_documents/_search",
            config.opensearch_url.as_ref().unwrap()
        );
        let search_resp: serde_json::Value = reqwest::Client::new()
            .post(&search_url)
            .json(&serde_json::json!({ "query": { "match": { "content_tokenized": "형태소" } } }))
            .send()
            .await
            .expect("search request failed")
            .json()
            .await
            .expect("search response was not valid JSON");
        let hits = search_resp["hits"]["total"]["value"].as_i64().unwrap_or(0);
        assert_eq!(
            hits, 1,
            "expected exactly one hit for a pre-tokenized morpheme term, got response: {}",
            search_resp
        );

        // The un-tokenized "content" field must retain the ORIGINAL raw
        // (un-spaced) text — proving the write-path fix for the HIGH-severity
        // architect finding that reindex previously overwrote _source with
        // tokenized text, corrupting display data.
        let hit_source = &search_resp["hits"]["hits"][0]["_source"];
        assert_eq!(
            hit_source["content"].as_str().unwrap_or(""),
            raw_content,
            "content field in _source must remain the original readable text, not the tokenized copy"
        );

        // Query-side proof (H1 fix): SearchService::search() must itself
        // tokenize an UN-SPACED raw Korean query the same way documents are
        // tokenized, or the query would never match the segmented morpheme
        // index (the exact query/document asymmetry an architect review
        // caught: the mechanism existed but the production /api/search
        // handler wasn't wired to use it). Construct the service WITH the
        // tokenizer (mirroring the fixed routes/search.rs::search handler)
        // and search for the same un-spaced text used to build the index.
        let search_service_with_tokenizer = SearchService::with_korean_tokenizer(
            &config,
            crate::services::load_shared_korean_tokenizer(),
        )
        .await
        .expect("failed to construct SearchService with korean tokenizer");
        let query_results = search_service_with_tokenizer
            .search(
                crate::models::SearchQuery {
                    q: raw_content.to_string(),
                    page: None,
                    limit: None,
                    category_id: None,
                    tags: None,
                    date_from: None,
                    date_to: None,
                    sort_by: None,
                    sort_order: None,
                },
                Some(1),
            )
            .await
            .expect("SearchService::search must succeed");
        assert!(
            query_results.total >= 1,
            "SearchService::search with an un-spaced raw Korean query must match the tokenized index (H1 fix); got 0 hits, total={}",
            query_results.total
        );
        assert!(
            query_results.hits.iter().any(|h| h.title == "한국어 형태소 분석기"),
            "expected the indexed fixture document (matched by title) among query results, got: {:?}",
            query_results.hits
        );

        // Cleanup.
        let _ = service.delete_document("c2-mecab-ko-fixture").await;
        let _ = reqwest::Client::new().delete(&delete_url).send().await;
    }
}
