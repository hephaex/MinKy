//! Integration / model tests for the knowledge graph data types.
//!
//! These tests exercise serialization, query defaults, and business logic
//! without requiring a database connection.

mod common;

use minky::models::knowledge_graph::{
    ExpertiseLevel, GraphEdge, GraphNode, KnowledgeGraph, KnowledgeGraphMeta,
    KnowledgeGraphQuery, NodeType,
};

// ---------------------------------------------------------------------------
// NodeType tests
// ---------------------------------------------------------------------------

#[test]
fn node_type_display_all_variants() {
    let cases = [
        (NodeType::Document, "document"),
        (NodeType::Topic, "topic"),
        (NodeType::Technology, "technology"),
        (NodeType::Person, "person"),
        (NodeType::Insight, "insight"),
    ];

    for (variant, expected) in cases {
        assert_eq!(variant.to_string(), expected);
    }
}

#[test]
fn node_type_serde_roundtrip() {
    let variants = [
        NodeType::Document,
        NodeType::Topic,
        NodeType::Technology,
        NodeType::Person,
        NodeType::Insight,
    ];

    for variant in variants {
        let serialized = serde_json::to_string(&variant).unwrap();
        let deserialized: NodeType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, variant);
    }
}

// ---------------------------------------------------------------------------
// ExpertiseLevel tests
// ---------------------------------------------------------------------------

#[test]
fn expertise_level_boundary_values() {
    // Exactly at boundary
    assert_eq!(ExpertiseLevel::from_doc_count(0), ExpertiseLevel::Beginner);
    assert_eq!(ExpertiseLevel::from_doc_count(2), ExpertiseLevel::Beginner);
    assert_eq!(ExpertiseLevel::from_doc_count(3), ExpertiseLevel::Intermediate);
    assert_eq!(ExpertiseLevel::from_doc_count(7), ExpertiseLevel::Intermediate);
    assert_eq!(ExpertiseLevel::from_doc_count(8), ExpertiseLevel::Advanced);
    assert_eq!(ExpertiseLevel::from_doc_count(15), ExpertiseLevel::Advanced);
    assert_eq!(ExpertiseLevel::from_doc_count(16), ExpertiseLevel::Expert);
}

#[test]
fn expertise_level_serde_roundtrip() {
    let levels = [
        ExpertiseLevel::Beginner,
        ExpertiseLevel::Intermediate,
        ExpertiseLevel::Advanced,
        ExpertiseLevel::Expert,
    ];

    for level in levels {
        let serialized = serde_json::to_string(&level).unwrap();
        let deserialized: ExpertiseLevel = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, level);
    }
}

// ---------------------------------------------------------------------------
// KnowledgeGraphQuery tests
// ---------------------------------------------------------------------------

#[test]
fn knowledge_graph_query_defaults_are_reasonable() {
    let q = KnowledgeGraphQuery::default();

    let threshold = q.threshold.unwrap_or(0.0);
    assert!(
        (0.0..=1.0).contains(&threshold),
        "threshold must be in [0, 1]"
    );

    let max_edges = q.max_edges.unwrap_or(0);
    assert!(max_edges > 0, "max_edges must be positive");
    assert!(max_edges <= 20, "max_edges must not exceed 20");

    assert!(q.include_topics, "topics enabled by default");
    assert!(q.include_technologies, "technologies enabled by default");

    let max_docs = q.max_documents.unwrap_or(0);
    assert!(max_docs > 0, "max_documents must be positive");
}

// ---------------------------------------------------------------------------
// GraphNode serialization tests
// ---------------------------------------------------------------------------

#[test]
fn graph_node_document_id_is_omitted_when_none() {
    let node = GraphNode {
        id: "topic-rust".into(),
        label: "Rust".into(),
        node_type: NodeType::Topic,
        document_id: None,
        document_count: 5,
        summary: None,
        topics: Vec::new(),
    };

    let json = serde_json::to_value(&node).unwrap();
    assert!(json.get("document_id").is_none(), "document_id should be absent");
    assert!(json.get("summary").is_none(), "summary should be absent when None");
}

#[test]
fn graph_node_document_id_present_when_some() {
    let node = GraphNode {
        id: "doc-123".into(),
        label: "My Doc".into(),
        node_type: NodeType::Document,
        document_id: Some("uuid-here".into()),
        document_count: 0,
        summary: Some("A document".into()),
        topics: vec!["Rust".into()],
    };

    let json = serde_json::to_value(&node).unwrap();
    assert_eq!(json["document_id"], "uuid-here");
    assert_eq!(json["summary"], "A document");
    assert_eq!(json["topics"][0], "Rust");
}

// ---------------------------------------------------------------------------
// GraphEdge serialization tests
// ---------------------------------------------------------------------------

#[test]
fn graph_edge_label_omitted_when_none() {
    let edge = GraphEdge {
        id: "e1".into(),
        source: "a".into(),
        target: "b".into(),
        weight: 0.75,
        label: None,
    };

    let json = serde_json::to_value(&edge).unwrap();
    assert!(json.get("label").is_none());
    assert_eq!(json["weight"], 0.75);
}

#[test]
fn graph_edge_label_present_when_some() {
    let edge = GraphEdge {
        id: "e2".into(),
        source: "doc-1".into(),
        target: "doc-2".into(),
        weight: 0.85,
        label: Some("85%".into()),
    };

    let json = serde_json::to_value(&edge).unwrap();
    assert_eq!(json["label"], "85%");
}

// ---------------------------------------------------------------------------
// KnowledgeGraph structure tests
// ---------------------------------------------------------------------------

#[test]
fn knowledge_graph_serializes_all_fields() {
    let graph = KnowledgeGraph {
        nodes: vec![GraphNode {
            id: "doc-1".into(),
            label: "Test".into(),
            node_type: NodeType::Document,
            document_id: Some("uuid".into()),
            document_count: 0,
            summary: None,
            topics: Vec::new(),
        }],
        edges: vec![],
        meta: KnowledgeGraphMeta {
            total_documents: 1,
            similarity_threshold: 0.5,
            max_edges_per_node: 5,
        },
    };

    let json = serde_json::to_value(&graph).unwrap();
    assert!(json["nodes"].is_array());
    assert!(json["edges"].is_array());
    assert!(json["meta"].is_object());
    assert_eq!(json["meta"]["total_documents"], 1);
    assert_eq!(json["meta"]["similarity_threshold"], 0.5);
    assert_eq!(json["meta"]["max_edges_per_node"], 5);
}

#[test]
fn knowledge_graph_empty_nodes_and_edges() {
    let graph = KnowledgeGraph {
        nodes: vec![],
        edges: vec![],
        meta: KnowledgeGraphMeta {
            total_documents: 0,
            similarity_threshold: 0.7,
            max_edges_per_node: 3,
        },
    };

    let json = serde_json::to_value(&graph).unwrap();
    assert_eq!(json["nodes"].as_array().unwrap().len(), 0);
    assert_eq!(json["edges"].as_array().unwrap().len(), 0);
}
