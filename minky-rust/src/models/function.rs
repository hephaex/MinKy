use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Quarkdown-style function definition
/// Functions are called with syntax: .functionName {arg1} {arg2}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub id: Uuid,
    pub name: String,
    pub parameters: Vec<FunctionParameter>,
    pub body: FunctionBody,
    pub description: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Function parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParameter {
    pub name: String,
    pub param_type: ParameterType,
    pub required: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

/// Parameter types for function arguments
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    #[default]
    String,
    Number,
    Boolean,
    List,
    Date,
}

/// Function body - how the function is executed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum FunctionBody {
    /// Built-in function (e.g., "sum", "date", "var")
    Builtin(String),
    /// User-defined template with placeholders
    Template(String),
    /// AI-powered function using Claude
    AiPowered { prompt: String },
}

impl Default for FunctionBody {
    fn default() -> Self {
        Self::Builtin("noop".to_string())
    }
}

/// Parsed function call from document text
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: Vec<String>,
    pub start_pos: usize,
    pub end_pos: usize,
    pub raw_text: String,
}

/// Request to create a new function
#[derive(Debug, Deserialize)]
pub struct CreateFunctionRequest {
    pub name: String,
    pub parameters: Vec<FunctionParameter>,
    pub body: FunctionBody,
    pub description: Option<String>,
}

/// Request to update a function
#[derive(Debug, Deserialize)]
pub struct UpdateFunctionRequest {
    pub name: Option<String>,
    pub parameters: Option<Vec<FunctionParameter>>,
    pub body: Option<FunctionBody>,
    pub description: Option<String>,
}

/// Function execution context
#[derive(Debug, Clone, Default)]
pub struct FunctionContext {
    /// Variables available in scope
    pub variables: std::collections::HashMap<String, String>,
    /// Current document ID (if applicable)
    pub document_id: Option<Uuid>,
    /// Base path for file includes
    pub base_path: Option<String>,
}

/// Result of function execution
#[derive(Debug, Clone)]
pub struct FunctionResult {
    pub output: String,
    pub warnings: Vec<String>,
}

impl FunctionResult {
    pub fn ok(output: String) -> Self {
        Self {
            output,
            warnings: vec![],
        }
    }

    pub fn with_warning(output: String, warning: String) -> Self {
        Self {
            output,
            warnings: vec![warning],
        }
    }
}

/// Function summary for API responses
#[derive(Debug, Serialize)]
pub struct FunctionSummary {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub parameter_count: usize,
    pub is_builtin: bool,
}

impl From<&FunctionDefinition> for FunctionSummary {
    fn from(def: &FunctionDefinition) -> Self {
        Self {
            id: def.id,
            name: def.name.clone(),
            description: def.description.clone(),
            parameter_count: def.parameters.len(),
            is_builtin: matches!(def.body, FunctionBody::Builtin(_)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_call_creation() {
        let call = FunctionCall {
            name: "date".to_string(),
            arguments: vec!["%Y-%m-%d".to_string()],
            start_pos: 0,
            end_pos: 20,
            raw_text: ".date {%Y-%m-%d}".to_string(),
        };
        assert_eq!(call.name, "date");
        assert_eq!(call.arguments.len(), 1);
    }

    #[test]
    fn test_function_body_serialization() {
        let builtin = FunctionBody::Builtin("sum".to_string());
        let json = serde_json::to_string(&builtin).unwrap();
        assert!(json.contains("\"type\":\"Builtin\""));
        assert!(json.contains("sum"));

        let template = FunctionBody::Template("Hello {{name}}".to_string());
        let json = serde_json::to_string(&template).unwrap();
        assert!(json.contains("\"type\":\"Template\""));

        let ai = FunctionBody::AiPowered {
            prompt: "Summarize this".to_string(),
        };
        let json = serde_json::to_string(&ai).unwrap();
        assert!(json.contains("\"type\":\"AiPowered\""));
    }

    #[test]
    fn test_parameter_type_default() {
        let pt = ParameterType::default();
        assert!(matches!(pt, ParameterType::String));
    }

    #[test]
    fn test_function_context_default() {
        let ctx = FunctionContext::default();
        assert!(ctx.variables.is_empty());
        assert!(ctx.document_id.is_none());
        assert!(ctx.base_path.is_none());
    }

    #[test]
    fn test_function_result_ok() {
        let result = FunctionResult::ok("output".to_string());
        assert_eq!(result.output, "output");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_function_result_with_warning() {
        let result = FunctionResult::with_warning("output".to_string(), "warning".to_string());
        assert_eq!(result.output, "output");
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_function_summary_from_definition() {
        let def = FunctionDefinition {
            id: Uuid::new_v4(),
            name: "test_fn".to_string(),
            parameters: vec![FunctionParameter {
                name: "arg1".to_string(),
                param_type: ParameterType::String,
                required: true,
                default_value: None,
                description: None,
            }],
            body: FunctionBody::Builtin("test".to_string()),
            description: Some("Test function".to_string()),
            created_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let summary = FunctionSummary::from(&def);
        assert_eq!(summary.name, "test_fn");
        assert_eq!(summary.parameter_count, 1);
        assert!(summary.is_builtin);
    }
}
