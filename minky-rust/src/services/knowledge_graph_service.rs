//! Knowledge graph service
//!
//! Builds an in-memory knowledge graph from:
//! 1. Document similarity scores stored in `document_embeddings` (pgvector cosine distance)
//! 2. Topics and technologies extracted by the document understanding pipeline
//! 3. User (team member) document authorship

use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use std::collections::VecDeque;

use crate::error::Result;
use crate::models::knowledge_graph::{
    DocumentTopicRow, ExpertiseArea, ExpertiseLevel, GraphEdge, GraphNode, GraphPath,
    KnowledgeGraph, KnowledgeGraphMeta, KnowledgeGraphQuery, MemberExpertise, NodeType,
    PathQuery, SimilarityPairRow, TeamExpertiseMap, UniqueExpert,
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

    /// Find the shortest path between two nodes in the knowledge graph.
    pub async fn find_path(&self, query: &PathQuery) -> Result<GraphPath> {
        let max_depth = query.max_depth.unwrap_or(5).clamp(1, 10);

        // Build the full graph first
        let graph_query = KnowledgeGraphQuery::default();
        let graph = self.build_graph(&graph_query).await?;

        // Use BFS to find shortest path
        let path = find_path_bfs(&graph, &query.from, &query.to, max_depth as usize);
        Ok(path)
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

/// Find the shortest path between two nodes using BFS.
///
/// This is a pure function that operates on the in-memory graph structure.
pub fn find_path_bfs(
    graph: &KnowledgeGraph,
    from: &str,
    to: &str,
    max_depth: usize,
) -> GraphPath {
    // Build adjacency list from edges (undirected)
    let mut adjacency: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for edge in &graph.edges {
        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push((edge.target.clone(), edge.id.clone()));
        adjacency
            .entry(edge.target.clone())
            .or_default()
            .push((edge.source.clone(), edge.id.clone()));
    }

    // Build node lookup
    let node_map: HashMap<String, &GraphNode> = graph
        .nodes
        .iter()
        .map(|n| (n.id.clone(), n))
        .collect();

    // Check if source and target exist
    if !node_map.contains_key(from) || !node_map.contains_key(to) {
        return GraphPath {
            node_ids: vec![],
            nodes: vec![],
            edges: vec![],
            length: 0,
            found: false,
        };
    }

    // BFS with path tracking
    let mut visited: HashMap<String, (String, String)> = HashMap::new(); // node -> (parent, edge_id)
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();

    queue.push_back((from.to_string(), 0));
    visited.insert(from.to_string(), (String::new(), String::new()));

    while let Some((current, depth)) = queue.pop_front() {
        if current == to {
            // Reconstruct path
            return reconstruct_path(&visited, &node_map, &graph.edges, from, to);
        }

        if depth >= max_depth {
            continue;
        }

        if let Some(neighbors) = adjacency.get(&current) {
            for (neighbor, edge_id) in neighbors {
                if !visited.contains_key(neighbor) {
                    visited.insert(neighbor.clone(), (current.clone(), edge_id.clone()));
                    queue.push_back((neighbor.clone(), depth + 1));
                }
            }
        }
    }

    // No path found
    GraphPath {
        node_ids: vec![],
        nodes: vec![],
        edges: vec![],
        length: 0,
        found: false,
    }
}

/// Reconstruct the path from BFS visited map.
fn reconstruct_path(
    visited: &HashMap<String, (String, String)>,
    node_map: &HashMap<String, &GraphNode>,
    all_edges: &[GraphEdge],
    _from: &str,
    to: &str,
) -> GraphPath {
    let edge_map: HashMap<String, &GraphEdge> = all_edges
        .iter()
        .map(|e| (e.id.clone(), e))
        .collect();

    let mut node_ids: Vec<String> = Vec::new();
    let mut edge_ids: Vec<String> = Vec::new();
    let mut current = to.to_string();

    // Walk backwards from target to source
    while !current.is_empty() {
        node_ids.push(current.clone());
        if let Some((parent, edge_id)) = visited.get(&current) {
            if !edge_id.is_empty() {
                edge_ids.push(edge_id.clone());
            }
            current = parent.clone();
        } else {
            break;
        }
    }

    node_ids.reverse();
    edge_ids.reverse();

    // Build node and edge lists
    let nodes: Vec<GraphNode> = node_ids
        .iter()
        .filter_map(|id| node_map.get(id).map(|n| (*n).clone()))
        .collect();

    let edges: Vec<GraphEdge> = edge_ids
        .iter()
        .filter_map(|id| edge_map.get(id).map(|e| (*e).clone()))
        .collect();

    let length = edges.len() as i32;

    GraphPath {
        node_ids,
        nodes,
        edges,
        length,
        found: true,
    }
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

    // -- find_path_bfs --

    fn make_test_graph() -> KnowledgeGraph {
        // Create a simple graph: A -- B -- C -- D
        //                              \       /
        //                               E ----
        let nodes = vec![
            GraphNode {
                id: "A".into(),
                label: "Node A".into(),
                node_type: NodeType::Document,
                document_id: None,
                document_count: 0,
                summary: None,
                topics: vec![],
            },
            GraphNode {
                id: "B".into(),
                label: "Node B".into(),
                node_type: NodeType::Topic,
                document_id: None,
                document_count: 0,
                summary: None,
                topics: vec![],
            },
            GraphNode {
                id: "C".into(),
                label: "Node C".into(),
                node_type: NodeType::Document,
                document_id: None,
                document_count: 0,
                summary: None,
                topics: vec![],
            },
            GraphNode {
                id: "D".into(),
                label: "Node D".into(),
                node_type: NodeType::Document,
                document_id: None,
                document_count: 0,
                summary: None,
                topics: vec![],
            },
            GraphNode {
                id: "E".into(),
                label: "Node E".into(),
                node_type: NodeType::Technology,
                document_id: None,
                document_count: 0,
                summary: None,
                topics: vec![],
            },
        ];

        let edges = vec![
            GraphEdge {
                id: "e1".into(),
                source: "A".into(),
                target: "B".into(),
                weight: 0.9,
                label: None,
            },
            GraphEdge {
                id: "e2".into(),
                source: "B".into(),
                target: "C".into(),
                weight: 0.8,
                label: None,
            },
            GraphEdge {
                id: "e3".into(),
                source: "C".into(),
                target: "D".into(),
                weight: 0.7,
                label: None,
            },
            GraphEdge {
                id: "e4".into(),
                source: "B".into(),
                target: "E".into(),
                weight: 0.85,
                label: None,
            },
            GraphEdge {
                id: "e5".into(),
                source: "E".into(),
                target: "D".into(),
                weight: 0.75,
                label: None,
            },
        ];

        KnowledgeGraph {
            nodes,
            edges,
            meta: KnowledgeGraphMeta {
                total_documents: 3,
                similarity_threshold: 0.5,
                max_edges_per_node: 5,
            },
        }
    }

    #[test]
    fn test_find_path_bfs_direct_connection() {
        let graph = make_test_graph();
        let path = find_path_bfs(&graph, "A", "B", 5);

        assert!(path.found);
        assert_eq!(path.length, 1);
        assert_eq!(path.node_ids, vec!["A", "B"]);
        assert_eq!(path.edges.len(), 1);
        assert_eq!(path.edges[0].id, "e1");
    }

    #[test]
    fn test_find_path_bfs_multi_hop() {
        let graph = make_test_graph();
        let path = find_path_bfs(&graph, "A", "D", 10);

        assert!(path.found);
        // Shortest path is A -> B -> E -> D (3 edges) or A -> B -> C -> D (3 edges)
        assert_eq!(path.length, 3);
        assert_eq!(path.node_ids.len(), 4);
        assert_eq!(path.node_ids[0], "A");
        assert_eq!(path.node_ids[3], "D");
    }

    #[test]
    fn test_find_path_bfs_same_node() {
        let graph = make_test_graph();
        let path = find_path_bfs(&graph, "A", "A", 5);

        assert!(path.found);
        assert_eq!(path.length, 0);
        assert_eq!(path.node_ids, vec!["A"]);
        assert!(path.edges.is_empty());
    }

    #[test]
    fn test_find_path_bfs_node_not_found() {
        let graph = make_test_graph();
        let path = find_path_bfs(&graph, "A", "Z", 5);

        assert!(!path.found);
        assert_eq!(path.length, 0);
        assert!(path.node_ids.is_empty());
    }

    #[test]
    fn test_find_path_bfs_max_depth_exceeded() {
        let graph = make_test_graph();
        // A -> D requires 3 hops, but we limit to 2
        let path = find_path_bfs(&graph, "A", "D", 2);

        assert!(!path.found);
    }

    #[test]
    fn test_find_path_bfs_disconnected_nodes() {
        // Graph with isolated node
        let mut graph = make_test_graph();
        graph.nodes.push(GraphNode {
            id: "ISOLATED".into(),
            label: "Isolated".into(),
            node_type: NodeType::Document,
            document_id: None,
            document_count: 0,
            summary: None,
            topics: vec![],
        });

        let path = find_path_bfs(&graph, "A", "ISOLATED", 10);

        assert!(!path.found);
    }
}
