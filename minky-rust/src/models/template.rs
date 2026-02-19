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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_type_default_is_text() {
        assert!(matches!(VariableType::default(), VariableType::Text));
    }

    #[test]
    fn test_variable_type_serde_all_variants() {
        let types = [
            (VariableType::Text, "text"),
            (VariableType::Number, "number"),
            (VariableType::Date, "date"),
            (VariableType::Select, "select"),
            (VariableType::MultiSelect, "multiselect"),
            (VariableType::Boolean, "boolean"),
        ];
        for (vt, expected) in &types {
            let json = serde_json::to_value(vt).unwrap();
            assert_eq!(json, *expected);
        }
    }

    #[test]
    fn test_template_variable_required_field() {
        let var = TemplateVariable {
            name: "author".to_string(),
            description: Some("Document author".to_string()),
            default_value: None,
            required: true,
            var_type: VariableType::Text,
        };
        assert!(var.required);
        assert_eq!(var.name, "author");
    }

    #[test]
    fn test_template_variable_with_default_value() {
        let var = TemplateVariable {
            name: "status".to_string(),
            description: None,
            default_value: Some("draft".to_string()),
            required: false,
            var_type: VariableType::Select,
        };
        assert!(!var.required);
        assert_eq!(var.default_value.as_deref(), Some("draft"));
    }

    #[test]
    fn test_template_variable_serde_roundtrip() {
        let var = TemplateVariable {
            name: "count".to_string(),
            description: None,
            default_value: Some("0".to_string()),
            required: false,
            var_type: VariableType::Number,
        };
        let json = serde_json::to_string(&var).unwrap();
        let back: TemplateVariable = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "count");
        assert!(matches!(back.var_type, VariableType::Number));
    }

    #[test]
    fn test_create_template_optional_fields_absent() {
        let json = "{\"name\": \"Meeting Notes\", \"content\": \"## Meeting\\n\\n{{notes}}\"}";
        let ct: CreateTemplate = serde_json::from_str(json).unwrap();
        assert_eq!(ct.name, "Meeting Notes");
        assert!(ct.variables.is_none());
        assert!(ct.tags.is_none());
        assert!(ct.is_public.is_none());
    }

    #[test]
    fn test_create_template_with_variables() {
        let json = r#"{
            "name": "Sprint Report",
            "content": "Sprint {{sprint_number}}",
            "variables": [
                {"name": "sprint_number", "required": true, "var_type": "number"}
            ]
        }"#;
        let ct: CreateTemplate = serde_json::from_str(json).unwrap();
        let vars = ct.variables.unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "sprint_number");
        assert!(vars[0].required);
    }

    #[test]
    fn test_update_template_all_none() {
        let json = r#"{}"#;
        let ut: UpdateTemplate = serde_json::from_str(json).unwrap();
        assert!(ut.name.is_none());
        assert!(ut.content.is_none());
        assert!(ut.is_public.is_none());
    }

    #[test]
    fn test_apply_template_request_minimal() {
        let json = r#"{"template_id": 5}"#;
        let req: ApplyTemplateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.template_id, 5);
        assert!(req.variables.is_none());
        assert!(req.title.is_none());
    }

    #[test]
    fn test_template_query_all_none() {
        let json = r#"{}"#;
        let q: TemplateQuery = serde_json::from_str(json).unwrap();
        assert!(q.page.is_none());
        assert!(q.limit.is_none());
        assert!(q.search.is_none());
    }
}
