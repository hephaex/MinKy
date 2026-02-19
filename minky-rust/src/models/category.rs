use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Category model representing the categories table
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
    pub user_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for creating a new category
#[derive(Debug, Deserialize)]
pub struct CreateCategory {
    pub name: String,
    pub parent_id: Option<i32>,
}

/// DTO for updating a category
#[derive(Debug, Deserialize)]
pub struct UpdateCategory {
    pub name: Option<String>,
    pub parent_id: Option<i32>,
}

/// Category with document count and children
#[derive(Debug, Serialize)]
pub struct CategoryTree {
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
    pub document_count: i64,
    pub children: Vec<CategoryTree>,
}

impl CategoryTree {
    /// Build a tree structure from flat categories
    pub fn build_tree(categories: Vec<CategoryWithCount>, parent_id: Option<i32>) -> Vec<Self> {
        categories
            .iter()
            .filter(|c| c.parent_id == parent_id)
            .map(|c| CategoryTree {
                id: c.id,
                name: c.name.clone(),
                parent_id: c.parent_id,
                document_count: c.document_count,
                children: Self::build_tree(categories.clone(), Some(c.id)),
            })
            .collect()
    }
}

/// Category with document count
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CategoryWithCount {
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
    pub user_id: i32,
    pub document_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_category(id: i32, name: &str, parent_id: Option<i32>) -> CategoryWithCount {
        CategoryWithCount {
            id,
            name: name.to_string(),
            parent_id,
            user_id: 1,
            document_count: 0,
        }
    }

    #[test]
    fn test_build_tree_empty() {
        let result = CategoryTree::build_tree(vec![], None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_build_tree_flat_roots() {
        let cats = vec![
            make_category(1, "Engineering", None),
            make_category(2, "Marketing", None),
        ];
        let tree = CategoryTree::build_tree(cats, None);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].name, "Engineering");
        assert_eq!(tree[1].name, "Marketing");
        assert!(tree[0].children.is_empty());
    }

    #[test]
    fn test_build_tree_with_children() {
        let cats = vec![
            make_category(1, "Engineering", None),
            make_category(2, "Backend", Some(1)),
            make_category(3, "Frontend", Some(1)),
            make_category(4, "Marketing", None),
        ];
        let tree = CategoryTree::build_tree(cats, None);

        // Two root nodes
        assert_eq!(tree.len(), 2);

        // Engineering has two children
        let engineering = tree.iter().find(|c| c.name == "Engineering").unwrap();
        assert_eq!(engineering.children.len(), 2);

        // Marketing has no children
        let marketing = tree.iter().find(|c| c.name == "Marketing").unwrap();
        assert!(marketing.children.is_empty());
    }

    #[test]
    fn test_build_tree_nested_hierarchy() {
        let cats = vec![
            make_category(1, "Root", None),
            make_category(2, "Level1", Some(1)),
            make_category(3, "Level2", Some(2)),
        ];
        let tree = CategoryTree::build_tree(cats, None);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].name, "Level1");
        assert_eq!(tree[0].children[0].children.len(), 1);
        assert_eq!(tree[0].children[0].children[0].name, "Level2");
    }

    #[test]
    fn test_build_tree_preserves_document_count() {
        let mut cat = make_category(1, "Docs", None);
        cat.document_count = 42;
        let tree = CategoryTree::build_tree(vec![cat], None);

        assert_eq!(tree[0].document_count, 42);
    }
}
