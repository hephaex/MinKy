//! Hierarchical Context Model
//!
//! QMD-inspired hierarchical context annotations for collections and documents.
//! Allows LLMs to understand semantic context beyond just document content.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Collection Context
// ---------------------------------------------------------------------------

/// A collection of documents with semantic context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    /// Unique identifier
    pub id: Uuid,
    /// Collection name
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Glob patterns for file matching
    pub patterns: Vec<String>,
    /// Parent collection (for hierarchy)
    pub parent_id: Option<Uuid>,
    /// Semantic context annotations
    pub context: CollectionContext,
    /// Number of documents
    pub document_count: i64,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Semantic context for a collection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CollectionContext {
    /// Brief description of what this collection contains
    pub summary: Option<String>,
    /// Topics covered in this collection
    pub topics: Vec<String>,
    /// Target audience or roles
    pub audience: Vec<String>,
    /// Related collections
    pub related_collections: Vec<Uuid>,
    /// Custom metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Context Annotation
// ---------------------------------------------------------------------------

/// A context annotation that can be attached to collections or documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAnnotation {
    /// Unique identifier
    pub id: Uuid,
    /// Target type (collection or document)
    pub target_type: ContextTargetType,
    /// Target ID (collection_id or document_id)
    pub target_id: Uuid,
    /// Context path (e.g., "minky://notes/project-x")
    pub context_path: String,
    /// Context text description
    pub context_text: String,
    /// Context type
    pub context_type: ContextType,
    /// Priority for context selection (higher = more important)
    pub priority: i32,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// Target type for context annotation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextTargetType {
    Collection,
    Document,
}

/// Type of context annotation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextType {
    /// General description
    Description,
    /// Purpose or use case
    Purpose,
    /// Target audience
    Audience,
    /// Related topics
    Topic,
    /// Temporal context (when relevant)
    Temporal,
    /// Source/origin information
    Source,
    /// Custom annotation
    Custom,
}

// ---------------------------------------------------------------------------
// Context Path
// ---------------------------------------------------------------------------

/// Parsed context path (e.g., "minky://notes/project-x/meetings")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPath {
    /// Full path string
    pub path: String,
    /// Path segments
    pub segments: Vec<String>,
    /// Depth (number of segments)
    pub depth: usize,
}

impl ContextPath {
    /// Parse a context path string
    pub fn parse(path: &str) -> Self {
        let normalized = path
            .trim()
            .trim_start_matches("minky://")
            .trim_end_matches('/');

        let segments: Vec<String> = normalized
            .split('/')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        let depth = segments.len();

        Self {
            path: path.to_string(),
            segments,
            depth,
        }
    }

    /// Get the parent path
    pub fn parent(&self) -> Option<Self> {
        if self.segments.len() <= 1 {
            return None;
        }

        let parent_segments = &self.segments[..self.segments.len() - 1];
        let parent_path = format!("minky://{}", parent_segments.join("/"));

        Some(Self::parse(&parent_path))
    }

    /// Check if this path is a descendant of another
    pub fn is_descendant_of(&self, ancestor: &ContextPath) -> bool {
        if self.segments.len() <= ancestor.segments.len() {
            return false;
        }

        self.segments
            .iter()
            .zip(ancestor.segments.iter())
            .all(|(a, b)| a == b)
    }

    /// Get the leaf name (last segment)
    pub fn leaf(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }
}

// ---------------------------------------------------------------------------
// Hierarchical Context Tree
// ---------------------------------------------------------------------------

/// A node in the context tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTreeNode {
    /// Node path
    pub path: String,
    /// Node name (leaf segment)
    pub name: String,
    /// Context text
    pub context: Option<String>,
    /// Child nodes
    pub children: Vec<ContextTreeNode>,
    /// Associated collection ID
    pub collection_id: Option<Uuid>,
    /// Document count at this level
    pub document_count: i64,
}

impl ContextTreeNode {
    /// Create an empty root node
    pub fn root() -> Self {
        Self {
            path: "minky://".to_string(),
            name: "root".to_string(),
            context: Some("MinKy Knowledge Base".to_string()),
            children: vec![],
            collection_id: None,
            document_count: 0,
        }
    }

    /// Find a node by path
    pub fn find(&self, path: &ContextPath) -> Option<&ContextTreeNode> {
        if self.path == path.path {
            return Some(self);
        }

        for child in &self.children {
            if let Some(found) = child.find(path) {
                return Some(found);
            }
        }

        None
    }

    /// Get all leaf nodes
    pub fn leaves(&self) -> Vec<&ContextTreeNode> {
        if self.children.is_empty() {
            return vec![self];
        }

        self.children.iter().flat_map(|c| c.leaves()).collect()
    }

    /// Count total nodes in the tree
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(|c| c.count()).sum::<usize>()
    }
}

// ---------------------------------------------------------------------------
// Context Builder for LLM Prompts
// ---------------------------------------------------------------------------

/// Build context strings for LLM prompts
pub struct ContextBuilder;

impl ContextBuilder {
    /// Build a hierarchical context string for a document
    pub fn build_document_context(
        path: &ContextPath,
        annotations: &[ContextAnnotation],
    ) -> String {
        let mut parts = Vec::new();

        // Add path hierarchy
        parts.push(format!("Location: {}", path.path));

        // Add annotations by priority
        let mut sorted: Vec<_> = annotations.iter().collect();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        for annotation in sorted.iter().take(5) {
            let prefix = match annotation.context_type {
                ContextType::Description => "About",
                ContextType::Purpose => "Purpose",
                ContextType::Audience => "For",
                ContextType::Topic => "Topic",
                ContextType::Temporal => "When",
                ContextType::Source => "Source",
                ContextType::Custom => "Note",
            };
            parts.push(format!("{}: {}", prefix, annotation.context_text));
        }

        parts.join("\n")
    }

    /// Build a collection context summary
    pub fn build_collection_context(collection: &Collection) -> String {
        let mut parts = Vec::new();

        parts.push(format!("Collection: {}", collection.name));

        if let Some(desc) = &collection.description {
            parts.push(format!("Description: {}", desc));
        }

        if let Some(summary) = &collection.context.summary {
            parts.push(format!("Summary: {}", summary));
        }

        if !collection.context.topics.is_empty() {
            parts.push(format!("Topics: {}", collection.context.topics.join(", ")));
        }

        if !collection.context.audience.is_empty() {
            parts.push(format!("Audience: {}", collection.context.audience.join(", ")));
        }

        parts.push(format!("Documents: {}", collection.document_count));

        parts.join("\n")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_path_parse_simple() {
        let path = ContextPath::parse("minky://notes");
        assert_eq!(path.segments, vec!["notes"]);
        assert_eq!(path.depth, 1);
    }

    #[test]
    fn test_context_path_parse_nested() {
        let path = ContextPath::parse("minky://notes/project-x/meetings");
        assert_eq!(path.segments, vec!["notes", "project-x", "meetings"]);
        assert_eq!(path.depth, 3);
    }

    #[test]
    fn test_context_path_parse_without_prefix() {
        let path = ContextPath::parse("notes/project-x");
        assert_eq!(path.segments, vec!["notes", "project-x"]);
    }

    #[test]
    fn test_context_path_parent() {
        let path = ContextPath::parse("minky://notes/project-x/meetings");
        let parent = path.parent().unwrap();
        assert_eq!(parent.segments, vec!["notes", "project-x"]);
    }

    #[test]
    fn test_context_path_parent_of_root() {
        let path = ContextPath::parse("minky://notes");
        assert!(path.parent().is_none());
    }

    #[test]
    fn test_context_path_is_descendant() {
        let parent = ContextPath::parse("minky://notes");
        let child = ContextPath::parse("minky://notes/project-x/meetings");

        assert!(child.is_descendant_of(&parent));
        assert!(!parent.is_descendant_of(&child));
    }

    #[test]
    fn test_context_path_leaf() {
        let path = ContextPath::parse("minky://notes/project-x/meetings");
        assert_eq!(path.leaf(), Some("meetings"));
    }

    #[test]
    fn test_context_tree_node_root() {
        let root = ContextTreeNode::root();
        assert_eq!(root.name, "root");
        assert_eq!(root.count(), 1);
    }

    #[test]
    fn test_context_tree_node_count() {
        let mut root = ContextTreeNode::root();
        root.children.push(ContextTreeNode {
            path: "minky://a".to_string(),
            name: "a".to_string(),
            context: None,
            children: vec![],
            collection_id: None,
            document_count: 0,
        });
        root.children.push(ContextTreeNode {
            path: "minky://b".to_string(),
            name: "b".to_string(),
            context: None,
            children: vec![],
            collection_id: None,
            document_count: 0,
        });

        assert_eq!(root.count(), 3);
    }

    #[test]
    fn test_context_builder_document() {
        let path = ContextPath::parse("minky://notes/project-x");
        let annotations = vec![
            ContextAnnotation {
                id: Uuid::new_v4(),
                target_type: ContextTargetType::Document,
                target_id: Uuid::new_v4(),
                context_path: path.path.clone(),
                context_text: "Meeting notes for Project X".to_string(),
                context_type: ContextType::Description,
                priority: 10,
                created_at: Utc::now(),
            },
        ];

        let context = ContextBuilder::build_document_context(&path, &annotations);
        assert!(context.contains("Location: minky://notes/project-x"));
        assert!(context.contains("About: Meeting notes"));
    }

    #[test]
    fn test_collection_context_default() {
        let ctx = CollectionContext::default();
        assert!(ctx.summary.is_none());
        assert!(ctx.topics.is_empty());
    }

    #[test]
    fn test_context_type_serde() {
        let json = serde_json::to_string(&ContextType::Purpose).unwrap();
        assert_eq!(json, "\"purpose\"");

        let parsed: ContextType = serde_json::from_str("\"audience\"").unwrap();
        assert_eq!(parsed, ContextType::Audience);
    }

    #[test]
    fn test_context_target_type_serde() {
        let json = serde_json::to_string(&ContextTargetType::Document).unwrap();
        assert_eq!(json, "\"document\"");
    }
}
