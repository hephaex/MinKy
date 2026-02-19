//! Knowledge graph service
//!
//! Builds an in-memory knowledge graph from:
//! 1. Document similarity scores stored in `document_embeddings` (pgvector cosine distance)
//! 2. Topics and technologies extracted by the document understanding pipeline
//! 3. User (team member) document authorship

use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::Result;
use crate::models::knowledge_graph::{
    DocumentTopicRow, ExpertiseArea, ExpertiseLevel, GraphEdge, GraphNode, KnowledgeGraph,
    KnowledgeGraphMeta, KnowledgeGraphQuery, MemberExpertise, NodeType, SimilarityPairRow,
    TeamExpertiseMap, UniqueExpert,
};

/// Service for building and serving the knowledge graph
pub struct KnowledgeGraphService {
    pool: PgPool,
}

impl KnowledgeGraphService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Build the full knowledge graph for the given query parameters.
    pub async fn build_graph(&self, query: &KnowledgeGraphQuery) -> Result<KnowledgeGraph> {
        let threshold = query.threshold.unwrap_or(0.5).clamp(0.0, 1.0);
        let max_edges = query.max_edges.unwrap_or(5).clamp(1, 20);
        let max_documents = query.max_documents.unwrap_or(100).max(1);

        // 1. Load documents with their understanding (topics/technologies)
        let doc_rows = self.load_document_topics(max_documents).await?;

        // 2. Load pairwise similarity scores from pgvector
        let similarity_pairs = self
            .load_similarity_pairs(threshold, max_edges, max_documents)
            .await?;

        // 3. Assemble nodes
        let mut nodes: Vec<GraphNode> = Vec::new();
        let mut edges: Vec<GraphEdge> = Vec::new();

        // Map of uuid -> graph node id for document nodes
        let mut doc_node_ids: HashMap<Uuid, String> = HashMap::new();

        // Document nodes
        for row in &doc_rows {
            let node_id = format!("doc-{}", row.document_id);
            doc_node_ids.insert(row.document_id, node_id.clone());

            nodes.push(GraphNode {
                id: node_id,
                label: row
                    .title
                    .clone()
                    .unwrap_or_else(|| "Untitled".to_string()),
                node_type: NodeType::Document,
                document_id: Some(row.document_id.to_string()),
                document_count: 0,
                summary: row.summary.clone(),
                topics: row.topics.clone(),
            });
        }

        // Derived nodes (topics/technologies/insights) – deduplicated
        if query.include_topics || query.include_technologies || query.include_insights {
            let (topic_nodes, tech_nodes, insight_nodes) =
                build_derived_nodes_pure(&doc_rows, &doc_node_ids, &mut edges);

            if query.include_topics {
                nodes.extend(topic_nodes);
            }
            if query.include_technologies {
                nodes.extend(tech_nodes);
            }
            if query.include_insights {
                nodes.extend(insight_nodes);
            }
        }

        // Similarity edges between document nodes
        let mut edge_idx = edges.len();
        for pair in &similarity_pairs {
            let source_id = match doc_node_ids.get(&pair.source_id) {
                Some(id) => id.clone(),
                None => continue,
            };
            let target_id = match doc_node_ids.get(&pair.target_id) {
                Some(id) => id.clone(),
                None => continue,
            };

            let pct = (pair.similarity * 100.0).round() as u32;
            edges.push(GraphEdge {
                id: format!("sim-{}", edge_idx),
                source: source_id,
                target: target_id,
                weight: pair.similarity,
                label: Some(format!("{}%", pct)),
            });
            edge_idx += 1;
        }

        let total_documents = doc_rows.len() as i64;

        Ok(KnowledgeGraph {
            nodes,
            edges,
            meta: KnowledgeGraphMeta {
                total_documents,
                similarity_threshold: threshold,
                max_edges_per_node: max_edges,
            },
        })
    }

    /// Load documents with their AI understanding metadata.
    async fn load_document_topics(&self, limit: i64) -> Result<Vec<DocumentTopicRow>> {
        let rows = sqlx::query_as::<_, DocumentTopicRow>(
            r#"
            SELECT
                d.id           AS document_id,
                d.title        AS title,
                du.summary     AS summary,
                COALESCE(du.topics, '{}')        AS topics,
                COALESCE(du.technologies, '{}')  AS technologies,
                COALESCE(du.insights, '{}')      AS insights
            FROM documents d
            LEFT JOIN document_understanding du ON du.document_id = d.id
            ORDER BY d.created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Load pairwise cosine similarities from pgvector, filtered by threshold.
    /// Returns at most `max_edges` neighbours per source document.
    async fn load_similarity_pairs(
        &self,
        threshold: f64,
        max_edges: i32,
        max_documents: i64,
    ) -> Result<Vec<SimilarityPairRow>> {
        // Use a lateral join to get top-N similar documents per source
        let rows = sqlx::query_as::<_, SimilarityPairRow>(
            r#"
            SELECT
                src.document_id AS source_id,
                tgt.document_id AS target_id,
                (1.0 - (src.embedding <=> tgt.embedding))::float8 AS similarity
            FROM document_embeddings src
            JOIN LATERAL (
                SELECT
                    de2.document_id,
                    de2.embedding
                FROM document_embeddings de2
                WHERE de2.document_id != src.document_id
                  AND de2.model = src.model
                  AND (1.0 - (src.embedding <=> de2.embedding)) >= $1
                ORDER BY src.embedding <=> de2.embedding
                LIMIT $2
            ) tgt ON true
            WHERE src.document_id IN (
                SELECT document_id FROM document_embeddings LIMIT $3
            )
            "#,
        )
        .bind(threshold)
        .bind(max_edges)
        .bind(max_documents)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Build the team expertise map from document authorship and AI understanding.
    pub async fn build_team_expertise_map(&self) -> Result<TeamExpertiseMap> {
        // Get per-user technology/topic aggregates from documents they authored
        let user_tech_rows = sqlx::query_as::<_, UserTechRow>(
            r#"
            SELECT
                u.id           AS user_id,
                u.username     AS username,
                u.email        AS email,
                unnest(COALESCE(du.technologies, '{}')) AS area,
                COUNT(*)::bigint AS doc_count
            FROM documents d
            JOIN users u ON u.id = d.user_id
            LEFT JOIN document_understanding du ON du.document_id = d.id
            GROUP BY u.id, u.username, u.email, area
            HAVING unnest(COALESCE(du.technologies, '{}')) != ''
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        // Also aggregate topics separately
        let user_topic_rows = sqlx::query_as::<_, UserTechRow>(
            r#"
            SELECT
                u.id           AS user_id,
                u.username     AS username,
                u.email        AS email,
                unnest(COALESCE(du.topics, '{}')) AS area,
                COUNT(*)::bigint AS doc_count
            FROM documents d
            JOIN users u ON u.id = d.user_id
            LEFT JOIN document_understanding du ON du.document_id = d.id
            GROUP BY u.id, u.username, u.email, area
            HAVING unnest(COALESCE(du.topics, '{}')) != ''
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        // Document totals per user
        let user_doc_totals = sqlx::query_as::<_, UserDocTotal>(
            r#"
            SELECT u.id AS user_id, COUNT(d.id)::bigint AS total_documents
            FROM users u
            LEFT JOIN documents d ON d.user_id = u.id
            GROUP BY u.id
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let totals_map: HashMap<i32, i64> = user_doc_totals
            .into_iter()
            .map(|r| (r.user_id, r.total_documents))
            .collect();

        // Build per-member expertise structs
        let mut member_map: HashMap<i32, MemberExpertise> = HashMap::new();

        for row in user_tech_rows {
            let entry = member_map.entry(row.user_id).or_insert_with(|| MemberExpertise {
                user_id: row.user_id,
                username: row.username.clone(),
                email: row.email.clone(),
                expertise_areas: Vec::new(),
                total_documents: *totals_map.get(&row.user_id).unwrap_or(&0),
                top_technologies: Vec::new(),
                top_topics: Vec::new(),
            });

            entry.expertise_areas.push(ExpertiseArea {
                area: row.area.clone(),
                document_count: row.doc_count,
                level: ExpertiseLevel::from_doc_count(row.doc_count),
            });
            entry.top_technologies.push(row.area);
        }

        for row in user_topic_rows {
            let entry = member_map.entry(row.user_id).or_insert_with(|| MemberExpertise {
                user_id: row.user_id,
                username: row.username.clone(),
                email: row.email.clone(),
                expertise_areas: Vec::new(),
                total_documents: *totals_map.get(&row.user_id).unwrap_or(&0),
                top_technologies: Vec::new(),
                top_topics: Vec::new(),
            });

            entry.top_topics.push(row.area);
        }

        // Sort each member's expertise by doc_count descending, keep top 10
        for member in member_map.values_mut() {
            member
                .expertise_areas
                .sort_by(|a, b| b.document_count.cmp(&a.document_count));
            member.expertise_areas.truncate(10);
            member.top_technologies.truncate(5);
            member.top_topics.truncate(5);
        }

        let members: Vec<MemberExpertise> = member_map.into_values().collect();

        // Shared areas: technologies that appear for more than one member
        let mut area_member_count: HashMap<String, usize> = HashMap::new();
        for member in &members {
            for tech in &member.top_technologies {
                *area_member_count.entry(tech.clone()).or_insert(0) += 1;
            }
        }
        let shared_areas: Vec<String> = area_member_count
            .into_iter()
            .filter(|(_, c)| *c > 1)
            .map(|(area, _)| area)
            .collect();

        // Unique experts: areas that only one member has
        let mut area_single: HashMap<String, (i32, String)> = HashMap::new();
        for member in &members {
            for tech in &member.top_technologies {
                let entry = area_single.entry(tech.clone()).or_insert_with(|| {
                    (member.user_id, member.username.clone())
                });
                if entry.0 != member.user_id {
                    // Multiple members – remove from single map
                    area_single.insert(tech.clone(), (-1, String::new()));
                }
            }
        }
        let unique_experts: Vec<UniqueExpert> = area_single
            .into_iter()
            .filter(|(_, (uid, _))| *uid > 0)
            .map(|(area, (uid, name))| UniqueExpert {
                area,
                expert_user_id: uid,
                expert_name: name,
            })
            .collect();

        Ok(TeamExpertiseMap {
            members,
            shared_areas,
            unique_experts,
        })
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions (no PgPool dependency – testable without a DB)
// ---------------------------------------------------------------------------

/// Build derived (topic/technology/insight) nodes and the edges connecting
/// them to document nodes.
///
/// Extracted as a pure function so unit tests don't need a real `PgPool`.
///
/// Returns `(topic_nodes, tech_nodes, insight_nodes)`.
pub fn build_derived_nodes_pure(
    doc_rows: &[DocumentTopicRow],
    doc_node_ids: &HashMap<Uuid, String>,
    edges: &mut Vec<GraphEdge>,
) -> (Vec<GraphNode>, Vec<GraphNode>, Vec<GraphNode>) {
    let mut topic_map: HashMap<String, i64> = HashMap::new();
    let mut tech_map: HashMap<String, i64> = HashMap::new();
    let mut insight_map: HashMap<String, i64> = HashMap::new();

    let mut edge_idx = edges.len();

    for row in doc_rows {
        let doc_node_id = match doc_node_ids.get(&row.document_id) {
            Some(id) => id,
            None => continue,
        };

        for topic in &row.topics {
            let trimmed = topic.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            *topic_map.entry(trimmed.clone()).or_insert(0) += 1;

            let topic_id = format!("topic-{}", normalize_label(&trimmed));
            edges.push(GraphEdge {
                id: format!("e-{}", edge_idx),
                source: doc_node_id.clone(),
                target: topic_id,
                weight: 1.0,
                label: None,
            });
            edge_idx += 1;
        }

        for tech in &row.technologies {
            let trimmed = tech.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            *tech_map.entry(trimmed.clone()).or_insert(0) += 1;

            let tech_id = format!("tech-{}", normalize_label(&trimmed));
            edges.push(GraphEdge {
                id: format!("e-{}", edge_idx),
                source: doc_node_id.clone(),
                target: tech_id,
                weight: 1.0,
                label: None,
            });
            edge_idx += 1;
        }

        for insight in &row.insights {
            let trimmed = insight.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            *insight_map.entry(trimmed.clone()).or_insert(0) += 1;

            let insight_id = format!("insight-{}", normalize_label(&trimmed));
            edges.push(GraphEdge {
                id: format!("e-{}", edge_idx),
                source: doc_node_id.clone(),
                target: insight_id,
                weight: 0.8,
                label: None,
            });
            edge_idx += 1;
        }
    }

    let topic_nodes: Vec<GraphNode> = topic_map
        .into_iter()
        .map(|(label, count)| GraphNode {
            id: format!("topic-{}", normalize_label(&label)),
            label: label.clone(),
            node_type: NodeType::Topic,
            document_id: None,
            document_count: count,
            summary: None,
            topics: Vec::new(),
        })
        .collect();

    let tech_nodes: Vec<GraphNode> = tech_map
        .into_iter()
        .map(|(label, count)| GraphNode {
            id: format!("tech-{}", normalize_label(&label)),
            label: label.clone(),
            node_type: NodeType::Technology,
            document_id: None,
            document_count: count,
            summary: None,
            topics: Vec::new(),
        })
        .collect();

    let insight_nodes: Vec<GraphNode> = insight_map
        .into_iter()
        .map(|(label, count)| GraphNode {
            id: format!("insight-{}", normalize_label(&label)),
            label: label.clone(),
            node_type: NodeType::Insight,
            document_id: None,
            document_count: count,
            summary: None,
            topics: Vec::new(),
        })
        .collect();

    (topic_nodes, tech_nodes, insight_nodes)
}

/// Normalize a label to a safe node id fragment (lowercase, hyphens only).
pub fn normalize_label(label: &str) -> String {
    label
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ---------------------------------------------------------------------------
// Internal row types for SQL queries
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
struct UserTechRow {
    user_id: i32,
    username: String,
    email: String,
    area: String,
    doc_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct UserDocTotal {
    user_id: i32,
    total_documents: i64,
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- normalize_label --

    #[test]
    fn test_normalize_label_basic() {
        assert_eq!(normalize_label("PostgreSQL"), "postgresql");
        assert_eq!(normalize_label("Rust lang"), "rust-lang");
        assert_eq!(normalize_label("OpenAI API"), "openai-api");
    }

    #[test]
    fn test_normalize_label_special_chars() {
        assert_eq!(normalize_label("pgvector 0.4"), "pgvector-0-4");
        assert_eq!(normalize_label("  spaces  "), "spaces");
        assert_eq!(normalize_label("hello---world"), "hello-world");
    }

    #[test]
    fn test_normalize_label_empty() {
        assert_eq!(normalize_label(""), "");
        assert_eq!(normalize_label("   "), "");
    }

    // -- build_derived_nodes_pure --

    #[test]
    fn test_build_derived_nodes_empty() {
        let doc_rows: Vec<DocumentTopicRow> = Vec::new();
        let doc_node_ids: HashMap<Uuid, String> = HashMap::new();
        let mut edges: Vec<GraphEdge> = Vec::new();

        let (topics, techs, insights) =
            build_derived_nodes_pure(&doc_rows, &doc_node_ids, &mut edges);

        assert!(topics.is_empty());
        assert!(techs.is_empty());
        assert!(insights.is_empty());
        assert!(edges.is_empty());
    }

    #[test]
    fn test_build_derived_nodes_with_data() {
        let doc_id = Uuid::new_v4();
        let node_id = format!("doc-{}", doc_id);

        let doc_rows = vec![DocumentTopicRow {
            document_id: doc_id,
            title: Some("Test Doc".into()),
            summary: None,
            topics: vec!["Rust".into(), "Testing".into()],
            technologies: vec!["PostgreSQL".into()],
            insights: vec![],
        }];

        let mut doc_node_ids = HashMap::new();
        doc_node_ids.insert(doc_id, node_id.clone());

        let mut edges: Vec<GraphEdge> = Vec::new();

        let (topics, techs, _insights) =
            build_derived_nodes_pure(&doc_rows, &doc_node_ids, &mut edges);

        assert_eq!(topics.len(), 2); // Rust + Testing
        assert_eq!(techs.len(), 1); // PostgreSQL
        // Edges: 2 topic + 1 tech
        assert_eq!(edges.len(), 3);

        for edge in &edges {
            assert_eq!(edge.source, node_id);
        }
    }

    #[test]
    fn test_build_derived_nodes_deduplicates_topics() {
        let doc_id1 = Uuid::new_v4();
        let doc_id2 = Uuid::new_v4();

        let doc_rows = vec![
            DocumentTopicRow {
                document_id: doc_id1,
                title: Some("Doc 1".into()),
                summary: None,
                topics: vec!["Rust".into()],
                technologies: vec![],
                insights: vec![],
            },
            DocumentTopicRow {
                document_id: doc_id2,
                title: Some("Doc 2".into()),
                summary: None,
                topics: vec!["Rust".into()],
                technologies: vec![],
                insights: vec![],
            },
        ];

        let mut doc_node_ids = HashMap::new();
        doc_node_ids.insert(doc_id1, format!("doc-{}", doc_id1));
        doc_node_ids.insert(doc_id2, format!("doc-{}", doc_id2));

        let mut edges: Vec<GraphEdge> = Vec::new();

        let (topics, _techs, _insights) =
            build_derived_nodes_pure(&doc_rows, &doc_node_ids, &mut edges);

        // "Rust" is shared by 2 docs; only ONE topic node
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].document_count, 2);
        // Two edges (one per document)
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_build_derived_nodes_skips_empty_topics() {
        let doc_id = Uuid::new_v4();
        let node_id = format!("doc-{}", doc_id);

        let doc_rows = vec![DocumentTopicRow {
            document_id: doc_id,
            title: Some("Doc".into()),
            summary: None,
            topics: vec!["  ".into(), "".into(), "Valid".into()],
            technologies: vec![],
            insights: vec![],
        }];

        let mut doc_node_ids = HashMap::new();
        doc_node_ids.insert(doc_id, node_id.clone());

        let mut edges: Vec<GraphEdge> = Vec::new();

        let (topics, _techs, _insights) =
            build_derived_nodes_pure(&doc_rows, &doc_node_ids, &mut edges);

        // Only "Valid" should be present
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].label, "Valid");
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_build_derived_nodes_insight_weight_is_less_than_one() {
        let doc_id = Uuid::new_v4();
        let node_id = format!("doc-{}", doc_id);

        let doc_rows = vec![DocumentTopicRow {
            document_id: doc_id,
            title: None,
            summary: None,
            topics: vec![],
            technologies: vec![],
            insights: vec!["Key insight".into()],
        }];

        let mut doc_node_ids = HashMap::new();
        doc_node_ids.insert(doc_id, node_id);

        let mut edges: Vec<GraphEdge> = Vec::new();

        let (_topics, _techs, insights) =
            build_derived_nodes_pure(&doc_rows, &doc_node_ids, &mut edges);

        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].node_type, NodeType::Insight);
        // Insight edges have weight 0.8 (lower than topic/tech at 1.0)
        assert!((edges[0].weight - 0.8).abs() < 1e-6);
    }
}
