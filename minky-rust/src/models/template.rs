use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Document template
#[derive(Debug, Serialize)]
pub struct Template {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub category_id: Option<i32>,
    pub category_name: Option<String>,
    pub variables: Vec<TemplateVariable>,
    pub tags: Vec<String>,
    pub is_public: bool,
    pub usage_count: i64,
    pub created_by: i32,
    pub created_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Template variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: Option<String>,
    pub default_value: Option<String>,
    pub required: bool,
    pub var_type: VariableType,
}

/// Variable type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VariableType {
    #[default]
    Text,
    Number,
    Date,
    Select,
    MultiSelect,
    Boolean,
}

/// Create template request
#[derive(Debug, Deserialize)]
pub struct CreateTemplate {
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub category_id: Option<i32>,
    pub variables: Option<Vec<TemplateVariable>>,
    pub tags: Option<Vec<String>>,
    pub is_public: Option<bool>,
}

/// Update template request
#[derive(Debug, Deserialize)]
pub struct UpdateTemplate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub category_id: Option<i32>,
    pub variables: Option<Vec<TemplateVariable>>,
    pub tags: Option<Vec<String>>,
    pub is_public: Option<bool>,
}

/// Apply template request
#[derive(Debug, Deserialize)]
pub struct ApplyTemplateRequest {
    pub template_id: i32,
    pub variables: Option<serde_json::Value>,
    pub title: Option<String>,
    pub category_id: Option<i32>,
}

/// Template preview
#[derive(Debug, Serialize)]
pub struct TemplatePreview {
    pub content: String,
    pub title: Option<String>,
    pub missing_variables: Vec<String>,
}

/// Template list query
#[derive(Debug, Deserialize)]
pub struct TemplateQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub category_id: Option<i32>,
    pub search: Option<String>,
    pub is_public: Option<bool>,
}
