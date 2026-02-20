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

    #[test]
    fn test_category_structure() {
        let now = Utc::now();
        let cat = Category {
            id: 5,
            name: "Research".to_string(),
            parent_id: Some(1),
            user_id: 10,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(cat.id, 5);
        assert_eq!(cat.name, "Research");
        assert_eq!(cat.parent_id, Some(1));
        assert_eq!(cat.user_id, 10);
    }

    #[test]
    fn test_create_category() {
        let create = CreateCategory {
            name: "NewCategory".to_string(),
            parent_id: Some(2),
        };

        assert_eq!(create.name, "NewCategory");
        assert_eq!(create.parent_id, Some(2));
    }

    #[test]
    fn test_create_category_root() {
        let create = CreateCategory {
            name: "RootCategory".to_string(),
            parent_id: None,
        };

        assert_eq!(create.name, "RootCategory");
        assert_eq!(create.parent_id, None);
    }

    #[test]
    fn test_update_category_name_only() {
        let update = UpdateCategory {
            name: Some("Updated".to_string()),
            parent_id: None,
        };

        assert_eq!(update.name, Some("Updated".to_string()));
        assert_eq!(update.parent_id, None);
    }

    #[test]
    fn test_update_category_parent_only() {
        let update = UpdateCategory {
            name: None,
            parent_id: Some(5),
        };

        assert_eq!(update.name, None);
        assert_eq!(update.parent_id, Some(5));
    }

    #[test]
    fn test_category_with_count_structure() {
        let cat = CategoryWithCount {
            id: 3,
            name: "Active".to_string(),
            parent_id: None,
            user_id: 7,
            document_count: 15,
        };

        assert_eq!(cat.id, 3);
        assert_eq!(cat.document_count, 15);
    }

    #[test]
    fn test_category_tree_structure() {
        let tree = CategoryTree {
            id: 10,
            name: "Main".to_string(),
            parent_id: None,
            document_count: 50,
            children: vec![
                CategoryTree {
                    id: 11,
                    name: "Sub".to_string(),
                    parent_id: Some(10),
                    document_count: 20,
                    children: vec![],
                },
            ],
        };

        assert_eq!(tree.id, 10);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].parent_id, Some(10));
    }

    #[test]
    fn test_build_tree_wide_hierarchy() {
        let cats = vec![
            make_category(1, "A", None),
            make_category(2, "B", None),
            make_category(3, "C", None),
            make_category(4, "D", None),
            make_category(5, "E", None),
        ];
        let tree = CategoryTree::build_tree(cats, None);

        assert_eq!(tree.len(), 5);
        for node in &tree {
            assert!(node.children.is_empty());
        }
    }

    #[test]
    fn test_build_tree_deep_nesting() {
        let cats = vec![
            make_category(1, "L0", None),
            make_category(2, "L1", Some(1)),
            make_category(3, "L2", Some(2)),
            make_category(4, "L3", Some(3)),
            make_category(5, "L4", Some(4)),
        ];
        let tree = CategoryTree::build_tree(cats, None);

        assert_eq!(tree.len(), 1);
        let mut current = &tree[0];
        assert_eq!(current.name, "L0");

        for expected_name in &["L1", "L2", "L3", "L4"] {
            assert_eq!(current.children.len(), 1);
            current = &current.children[0];
            assert_eq!(current.name, *expected_name);
        }

        assert!(current.children.is_empty());
    }
}
