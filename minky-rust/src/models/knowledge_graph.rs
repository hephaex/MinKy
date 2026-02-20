//! Knowledge graph models for visualizing document relationships
//!
//! This module provides types for building and serving a knowledge graph
//! derived from document embeddings, document understanding data, and
//! team member activity patterns.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of a graph node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Document,
    Topic,
    Technology,
    Person,
    Insight,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Document => "document",
            Self::Topic => "topic",
            Self::Technology => "technology",
            Self::Person => "person",
            Self::Insight => "insight",
        };
        write!(f, "{}", s)
    }
}

/// A single node in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique identifier for the node (e.g. `doc-{uuid}` or `topic-{label}`)
    pub id: String,

    /// Human-readable label shown in the UI
    pub label: String,

    /// Node category determining colour and icon
    #[serde(rename = "type")]
    pub node_type: NodeType,

    /// Underlying document UUID for `Document` nodes (None for derived nodes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,

    /// Number of documents associated with this node
    pub document_count: i64,

    /// Optional short summary shown in the detail panel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Related topics / sub-labels for search and tooltip display
    #[serde(default)]
    pub topics: Vec<String>,
}

/// A directed or undirected edge between two graph nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Unique edge identifier
    pub id: String,

    /// Source node `id`
    pub source: String,

    /// Target node `id`
    pub target: String,

    /// Cosine similarity weight in [0, 1]
    pub weight: f64,

    /// Optional display label (e.g. "85%")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Complete knowledge graph response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    /// Metadata about how the graph was built
    pub meta: KnowledgeGraphMeta,
}

/// Metadata describing graph generation parameters and stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphMeta {
    /// Total documents included in the graph
    pub total_documents: i64,
    /// Minimum similarity threshold used to create edges
    pub similarity_threshold: f64,
    /// Maximum number of similar-document edges per document
    pub max_edges_per_node: i32,
}

/// Query parameters for the knowledge graph endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct KnowledgeGraphQuery {
    /// Minimum cosine similarity score for edges (0.0–1.0, default 0.5)
    pub threshold: Option<f64>,

    /// Maximum edges per document node (default 5, max 20)
    pub max_edges: Option<i32>,

    /// Include topic nodes derived from `document_understanding.topics`
    #[serde(default = "default_true")]
    pub include_topics: bool,

    /// Include technology nodes derived from `document_understanding.technologies`
    #[serde(default = "default_true")]
    pub include_technologies: bool,

    /// Include insight nodes derived from `document_understanding.insights`
    #[serde(default)]
    pub include_insights: bool,

    /// Maximum number of document nodes to include (default 100)
    pub max_documents: Option<i64>,
}

fn default_true() -> bool {
    true
}

impl Default for KnowledgeGraphQuery {
    fn default() -> Self {
        Self {
            threshold: Some(0.5),
            max_edges: Some(5),
            include_topics: true,
            include_technologies: true,
            include_insights: false,
            max_documents: Some(100),
        }
    }
}

/// Expertise level for a team member's skill
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExpertiseLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

impl ExpertiseLevel {
    /// Convert document count to expertise level
    pub fn from_doc_count(count: i64) -> Self {
        match count {
            0..=2 => Self::Beginner,
            3..=7 => Self::Intermediate,
            8..=15 => Self::Advanced,
            _ => Self::Expert,
        }
    }
}

impl std::fmt::Display for ExpertiseLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Beginner => "beginner",
            Self::Intermediate => "intermediate",
            Self::Advanced => "advanced",
            Self::Expert => "expert",
        };
        write!(f, "{}", s)
    }
}

/// A topic/technology area that a team member has expertise in
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertiseArea {
    pub area: String,
    pub document_count: i64,
    pub level: ExpertiseLevel,
}

/// Team member expertise profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberExpertise {
    pub user_id: i32,
    pub username: String,
    pub email: String,
    pub expertise_areas: Vec<ExpertiseArea>,
    pub total_documents: i64,
    pub top_technologies: Vec<String>,
    pub top_topics: Vec<String>,
}

/// Team expertise map response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamExpertiseMap {
    pub members: Vec<MemberExpertise>,
    pub shared_areas: Vec<String>,
    pub unique_experts: Vec<UniqueExpert>,
}

/// An area where only one team member has expertise
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueExpert {
    pub area: String,
    pub expert_user_id: i32,
    pub expert_name: String,
}

/// Row type for internal DB queries on document-topic relationships
#[derive(Debug, sqlx::FromRow)]
pub struct DocumentTopicRow {
    pub document_id: Uuid,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub topics: Vec<String>,
    pub technologies: Vec<String>,
    pub insights: Vec<String>,
}

/// Row type for similarity pairs from pgvector
#[derive(Debug, sqlx::FromRow)]
pub struct SimilarityPairRow {
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub similarity: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_type_display() {
        assert_eq!(NodeType::Document.to_string(), "document");
        assert_eq!(NodeType::Topic.to_string(), "topic");
        assert_eq!(NodeType::Technology.to_string(), "technology");
        assert_eq!(NodeType::Person.to_string(), "person");
        assert_eq!(NodeType::Insight.to_string(), "insight");
    }

    #[test]
    fn test_node_type_serde() {
        let json = serde_json::to_string(&NodeType::Topic).unwrap();
        assert_eq!(json, "\"topic\"");

        let deserialized: NodeType = serde_json::from_str("\"technology\"").unwrap();
        assert_eq!(deserialized, NodeType::Technology);
    }

    #[test]
    fn test_expertise_level_from_doc_count() {
        assert_eq!(ExpertiseLevel::from_doc_count(0), ExpertiseLevel::Beginner);
        assert_eq!(ExpertiseLevel::from_doc_count(2), ExpertiseLevel::Beginner);
        assert_eq!(ExpertiseLevel::from_doc_count(3), ExpertiseLevel::Intermediate);
        assert_eq!(ExpertiseLevel::from_doc_count(7), ExpertiseLevel::Intermediate);
        assert_eq!(ExpertiseLevel::from_doc_count(8), ExpertiseLevel::Advanced);
        assert_eq!(ExpertiseLevel::from_doc_count(15), ExpertiseLevel::Advanced);
        assert_eq!(ExpertiseLevel::from_doc_count(16), ExpertiseLevel::Expert);
        assert_eq!(ExpertiseLevel::from_doc_count(100), ExpertiseLevel::Expert);
    }

    #[test]
    fn test_expertise_level_display() {
        assert_eq!(ExpertiseLevel::Beginner.to_string(), "beginner");
        assert_eq!(ExpertiseLevel::Intermediate.to_string(), "intermediate");
        assert_eq!(ExpertiseLevel::Advanced.to_string(), "advanced");
        assert_eq!(ExpertiseLevel::Expert.to_string(), "expert");
    }

    #[test]
    fn test_knowledge_graph_query_default() {
        let q = KnowledgeGraphQuery::default();
        assert_eq!(q.threshold, Some(0.5));
        assert_eq!(q.max_edges, Some(5));
        assert!(q.include_topics);
        assert!(q.include_technologies);
        assert!(!q.include_insights);
        assert_eq!(q.max_documents, Some(100));
    }

    #[test]
    fn test_graph_node_serialization() {
        let node = GraphNode {
            id: "doc-123".into(),
            label: "Test Doc".into(),
            node_type: NodeType::Document,
            document_id: Some("uuid-abc".into()),
            document_count: 0,
            summary: Some("A test document".into()),
            topics: vec!["Rust".into(), "Testing".into()],
        };

        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["type"], "document");
        assert_eq!(json["label"], "Test Doc");
        assert_eq!(json["document_count"], 0);
        assert_eq!(json["topics"][0], "Rust");
    }

    #[test]
    fn test_graph_edge_serialization() {
        let edge = GraphEdge {
            id: "e1".into(),
            source: "doc-1".into(),
            target: "doc-2".into(),
            weight: 0.85,
            label: Some("85%".into()),
        };

        let json = serde_json::to_value(&edge).unwrap();
        assert_eq!(json["weight"], 0.85);
        assert_eq!(json["label"], "85%");
    }

    #[test]
    fn test_graph_edge_no_label_omitted() {
        let edge = GraphEdge {
            id: "e1".into(),
            source: "a".into(),
            target: "b".into(),
            weight: 0.5,
            label: None,
        };

        let json = serde_json::to_value(&edge).unwrap();
        // label should be omitted when None (skip_serializing_if)
        assert!(json.get("label").is_none());
    }
}
