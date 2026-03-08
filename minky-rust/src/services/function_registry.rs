use anyhow::{anyhow, Result};
use chrono::{Local, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;

use crate::models::{
    FunctionBody, FunctionCall, FunctionContext, FunctionDefinition, FunctionParameter,
    FunctionResult, ParameterType,
};
use crate::services::FunctionParser;

/// Registry of available functions (built-in and user-defined)
pub struct FunctionRegistry {
    functions: HashMap<String, FunctionDefinition>,
    parser: FunctionParser,
}

impl Default for FunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionRegistry {
    /// Create a new registry with built-in functions
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
            parser: FunctionParser::new(),
        };

        registry.register_builtins();
        registry
    }

    /// Register all built-in functions
    fn register_builtins(&mut self) {
        // .var {name} - Variable reference (handled by parser)
        self.register(create_builtin(
            "var",
            vec![param("name", ParameterType::String, true)],
            "var",
            "Reference a variable by name",
        ));

        // .date {format} - Current date
        self.register(create_builtin(
            "date",
            vec![param("format", ParameterType::String, false)],
            "date",
            "Insert current date with optional format",
        ));

        // .time {format} - Current time
        self.register(create_builtin(
            "time",
            vec![param("format", ParameterType::String, false)],
            "time",
            "Insert current time with optional format",
        ));

        // .datetime {format} - Current datetime
        self.register(create_builtin(
            "datetime",
            vec![param("format", ParameterType::String, false)],
            "datetime",
            "Insert current datetime with optional format",
        ));

        // .sum {numbers...} - Sum of numbers
        self.register(create_builtin(
            "sum",
            vec![param("numbers", ParameterType::List, true)],
            "sum",
            "Calculate sum of numbers",
        ));

        // .avg {numbers...} - Average of numbers
        self.register(create_builtin(
            "avg",
            vec![param("numbers", ParameterType::List, true)],
            "avg",
            "Calculate average of numbers",
        ));

        // .min {numbers...} - Minimum of numbers
        self.register(create_builtin(
            "min",
            vec![param("numbers", ParameterType::List, true)],
            "min",
            "Find minimum of numbers",
        ));

        // .max {numbers...} - Maximum of numbers
        self.register(create_builtin(
            "max",
            vec![param("numbers", ParameterType::List, true)],
            "max",
            "Find maximum of numbers",
        ));

        // .count {list} - Count items
        self.register(create_builtin(
            "count",
            vec![param("list", ParameterType::List, true)],
            "count",
            "Count items in a list",
        ));

        // .upper {text} - Uppercase
        self.register(create_builtin(
            "upper",
            vec![param("text", ParameterType::String, true)],
            "upper",
            "Convert text to uppercase",
        ));

        // .lower {text} - Lowercase
        self.register(create_builtin(
            "lower",
            vec![param("text", ParameterType::String, true)],
            "lower",
            "Convert text to lowercase",
        ));

        // .capitalize {text} - Capitalize
        self.register(create_builtin(
            "capitalize",
            vec![param("text", ParameterType::String, true)],
            "capitalize",
            "Capitalize first letter of text",
        ));

        // .trim {text} - Trim whitespace
        self.register(create_builtin(
            "trim",
            vec![param("text", ParameterType::String, true)],
            "trim",
            "Remove leading and trailing whitespace",
        ));

        // .include {path} - Include file content
        self.register(create_builtin(
            "include",
            vec![param("path", ParameterType::String, true)],
            "include",
            "Include content from another file",
        ));

        // .toc - Table of contents
        self.register(create_builtin(
            "toc",
            vec![],
            "toc",
            "Generate table of contents from headings",
        ));

        // .uuid - Generate UUID
        self.register(create_builtin("uuid", vec![], "uuid", "Generate a new UUID"));

        // .len {text} - String length
        self.register(create_builtin(
            "len",
            vec![param("text", ParameterType::String, true)],
            "len",
            "Get length of text",
        ));

        // .repeat {text} {count} - Repeat text
        self.register(create_builtin(
            "repeat",
            vec![
                param("text", ParameterType::String, true),
                param("count", ParameterType::Number, true),
            ],
            "repeat",
            "Repeat text n times",
        ));

        // .join {list} {separator} - Join list items
        self.register(create_builtin(
            "join",
            vec![
                param("list", ParameterType::List, true),
                param("separator", ParameterType::String, false),
            ],
            "join",
            "Join list items with separator",
        ));

        // .split {text} {delimiter} - Split text into list
        self.register(create_builtin(
            "split",
            vec![
                param("text", ParameterType::String, true),
                param("delimiter", ParameterType::String, false),
            ],
            "split",
            "Split text by delimiter",
        ));

        // .replace {text} {from} {to} - Replace text
        self.register(create_builtin(
            "replace",
            vec![
                param("text", ParameterType::String, true),
                param("from", ParameterType::String, true),
                param("to", ParameterType::String, true),
            ],
            "replace",
            "Replace occurrences in text",
        ));
    }

    /// Register a function
    pub fn register(&mut self, definition: FunctionDefinition) {
        self.functions.insert(definition.name.clone(), definition);
    }

    /// Get a function by name
    pub fn get(&self, name: &str) -> Option<&FunctionDefinition> {
        self.functions.get(name)
    }

    /// List all function names
    pub fn list(&self) -> Vec<&str> {
        self.functions.keys().map(|s| s.as_str()).collect()
    }

    /// List all function definitions
    pub fn definitions(&self) -> Vec<&FunctionDefinition> {
        self.functions.values().collect()
    }

    /// Check if function exists
    pub fn contains(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Execute a function call
    pub fn execute(&self, call: &FunctionCall, ctx: &FunctionContext) -> Result<FunctionResult> {
        let Some(definition) = self.get(&call.name) else {
            return Ok(FunctionResult::with_warning(
                format!("[unknown function: {}]", call.name),
                format!("Function '{}' not found", call.name),
            ));
        };

        match &definition.body {
            FunctionBody::Builtin(builtin_name) => {
                self.execute_builtin(builtin_name, &call.arguments, ctx)
            }
            FunctionBody::Template(template) => self.execute_template(template, &call.arguments),
            FunctionBody::AiPowered { prompt: _ } => {
                // AI-powered functions need external service
                Ok(FunctionResult::with_warning(
                    "[AI function - requires external service]".to_string(),
                    "AI-powered functions are not available in this context".to_string(),
                ))
            }
        }
    }

    /// Execute a built-in function
    fn execute_builtin(
        &self,
        name: &str,
        args: &[String],
        ctx: &FunctionContext,
    ) -> Result<FunctionResult> {
        match name {
            "var" => {
                // Variable substitution
                let var_name = args.first().map(|s| s.as_str()).unwrap_or("");
                let value = ctx
                    .variables
                    .get(var_name)
                    .cloned()
                    .unwrap_or_else(|| format!("[undefined: {}]", var_name));
                Ok(FunctionResult::ok(value))
            }

            "date" => {
                let format = args.first().map(|s| s.as_str()).unwrap_or("%Y-%m-%d");
                let date = Local::now().format(format).to_string();
                Ok(FunctionResult::ok(date))
            }

            "time" => {
                let format = args.first().map(|s| s.as_str()).unwrap_or("%H:%M:%S");
                let time = Local::now().format(format).to_string();
                Ok(FunctionResult::ok(time))
            }

            "datetime" => {
                let format = args
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or("%Y-%m-%d %H:%M:%S");
                let datetime = Local::now().format(format).to_string();
                Ok(FunctionResult::ok(datetime))
            }

            "sum" => {
                let sum: f64 = args
                    .iter()
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .sum();
                Ok(FunctionResult::ok(format_number(sum)))
            }

            "avg" => {
                let nums: Vec<f64> = args
                    .iter()
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .collect();
                if nums.is_empty() {
                    return Ok(FunctionResult::ok("0".to_string()));
                }
                let avg = nums.iter().sum::<f64>() / nums.len() as f64;
                Ok(FunctionResult::ok(format_number(avg)))
            }

            "min" => {
                let min = args
                    .iter()
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .fold(f64::INFINITY, f64::min);
                if min.is_infinite() {
                    Ok(FunctionResult::ok(String::new()))
                } else {
                    Ok(FunctionResult::ok(format_number(min)))
                }
            }

            "max" => {
                let max = args
                    .iter()
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .fold(f64::NEG_INFINITY, f64::max);
                if max.is_infinite() {
                    Ok(FunctionResult::ok(String::new()))
                } else {
                    Ok(FunctionResult::ok(format_number(max)))
                }
            }

            "count" => {
                let count: usize = args.iter().map(|s| s.split(',').count()).sum();
                Ok(FunctionResult::ok(count.to_string()))
            }

            "upper" => {
                let text = args.first().map(|s| s.to_uppercase()).unwrap_or_default();
                Ok(FunctionResult::ok(text))
            }

            "lower" => {
                let text = args.first().map(|s| s.to_lowercase()).unwrap_or_default();
                Ok(FunctionResult::ok(text))
            }

            "capitalize" => {
                let text = args.first().map(|s| capitalize_first(s)).unwrap_or_default();
                Ok(FunctionResult::ok(text))
            }

            "trim" => {
                let text = args.first().map(|s| s.trim().to_string()).unwrap_or_default();
                Ok(FunctionResult::ok(text))
            }

            "include" => {
                let path = args.first().ok_or_else(|| anyhow!("include requires a path"))?;

                // Resolve path relative to base_path if provided
                let full_path = if let Some(base) = &ctx.base_path {
                    Path::new(base).join(path)
                } else {
                    Path::new(path).to_path_buf()
                };

                match fs::read_to_string(&full_path) {
                    Ok(content) => Ok(FunctionResult::ok(content)),
                    Err(e) => Ok(FunctionResult::with_warning(
                        format!("[include error: {}]", path),
                        format!("Failed to include {}: {}", path, e),
                    )),
                }
            }

            "toc" => {
                // TOC requires document content - return placeholder
                Ok(FunctionResult::ok("[TOC]".to_string()))
            }

            "uuid" => Ok(FunctionResult::ok(Uuid::new_v4().to_string())),

            "len" => {
                let len = args.first().map(|s| s.len()).unwrap_or(0);
                Ok(FunctionResult::ok(len.to_string()))
            }

            "repeat" => {
                let text = args.first().map(|s| s.as_str()).unwrap_or("");
                let count: usize = args
                    .get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1)
                    .min(1000); // Limit to prevent abuse
                Ok(FunctionResult::ok(text.repeat(count)))
            }

            "join" => {
                let items: Vec<&str> = args
                    .first()
                    .map(|s| s.split(',').map(|i| i.trim()).collect())
                    .unwrap_or_default();
                let sep = args.get(1).map(|s| s.as_str()).unwrap_or(", ");
                Ok(FunctionResult::ok(items.join(sep)))
            }

            "split" => {
                let text = args.first().map(|s| s.as_str()).unwrap_or("");
                let delim = args.get(1).map(|s| s.as_str()).unwrap_or(",");
                let parts: Vec<&str> = text.split(delim).collect();
                Ok(FunctionResult::ok(parts.join(", ")))
            }

            "replace" => {
                let text = args.first().map(|s| s.as_str()).unwrap_or("");
                let from = args.get(1).map(|s| s.as_str()).unwrap_or("");
                let to = args.get(2).map(|s| s.as_str()).unwrap_or("");
                Ok(FunctionResult::ok(text.replace(from, to)))
            }

            _ => Ok(FunctionResult::with_warning(
                format!("[unknown builtin: {}]", name),
                format!("Built-in function '{}' not implemented", name),
            )),
        }
    }

    /// Execute a template function
    fn execute_template(&self, template: &str, args: &[String]) -> Result<FunctionResult> {
        let mut result = template.to_string();

        // Replace {{0}}, {{1}}, etc. with arguments
        for (i, arg) in args.iter().enumerate() {
            result = result.replace(&format!("{{{{{}}}}}", i), arg);
        }

        // Replace {{args}} with all arguments joined
        result = result.replace("{{args}}", &args.join(", "));

        Ok(FunctionResult::ok(result))
    }

    /// Expand all functions in a document
    pub fn expand_document(&self, content: &str, ctx: &FunctionContext) -> String {
        self.parser.expand(content, ctx, |call, ctx| {
            self.execute(call, ctx).unwrap_or_else(|e| {
                FunctionResult::with_warning(
                    format!("[error: {}]", e),
                    format!("Function execution failed: {}", e),
                )
            })
        })
    }

    /// Generate Table of Contents from markdown headings
    pub fn generate_toc(&self, content: &str) -> String {
        let heading_regex = regex::Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
        let mut toc = String::new();

        for line in content.lines() {
            if let Some(cap) = heading_regex.captures(line) {
                let level = cap.get(1).unwrap().as_str().len();
                let title = cap.get(2).unwrap().as_str();
                let anchor = title
                    .to_lowercase()
                    .replace(' ', "-")
                    .replace(|c: char| !c.is_alphanumeric() && c != '-', "");

                let indent = "  ".repeat(level - 1);
                toc.push_str(&format!("{}- [{}](#{})\n", indent, title, anchor));
            }
        }

        toc
    }
}

/// Helper to create a built-in function definition
fn create_builtin(
    name: &str,
    parameters: Vec<FunctionParameter>,
    builtin_name: &str,
    description: &str,
) -> FunctionDefinition {
    FunctionDefinition {
        id: Uuid::nil(),
        name: name.to_string(),
        parameters,
        body: FunctionBody::Builtin(builtin_name.to_string()),
        description: Some(description.to_string()),
        created_by: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Helper to create a function parameter
fn param(name: &str, param_type: ParameterType, required: bool) -> FunctionParameter {
    FunctionParameter {
        name: name.to_string(),
        param_type,
        required,
        default_value: None,
        description: None,
    }
}

/// Format a number (remove trailing zeros for integers)
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

/// Capitalize first letter
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> FunctionContext {
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "World".to_string());
        variables.insert("count".to_string(), "42".to_string());

        FunctionContext {
            variables,
            document_id: None,
            base_path: None,
        }
    }

    #[test]
    fn test_registry_has_builtins() {
        let registry = FunctionRegistry::new();
        assert!(registry.contains("date"));
        assert!(registry.contains("sum"));
        assert!(registry.contains("upper"));
    }

    #[test]
    fn test_execute_date() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();
        let call = FunctionCall {
            name: "date".to_string(),
            arguments: vec!["%Y".to_string()],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert_eq!(result.output, "2026"); // Assuming test runs in 2026
    }

    #[test]
    fn test_execute_sum() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();
        let call = FunctionCall {
            name: "sum".to_string(),
            arguments: vec!["10".to_string(), "20".to_string(), "30".to_string()],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert_eq!(result.output, "60");
    }

    #[test]
    fn test_execute_avg() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();
        let call = FunctionCall {
            name: "avg".to_string(),
            arguments: vec!["10".to_string(), "20".to_string(), "30".to_string()],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert_eq!(result.output, "20");
    }

    #[test]
    fn test_execute_upper() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();
        let call = FunctionCall {
            name: "upper".to_string(),
            arguments: vec!["hello".to_string()],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert_eq!(result.output, "HELLO");
    }

    #[test]
    fn test_execute_lower() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();
        let call = FunctionCall {
            name: "lower".to_string(),
            arguments: vec!["HELLO".to_string()],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert_eq!(result.output, "hello");
    }

    #[test]
    fn test_execute_capitalize() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();
        let call = FunctionCall {
            name: "capitalize".to_string(),
            arguments: vec!["hello world".to_string()],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert_eq!(result.output, "Hello world");
    }

    #[test]
    fn test_execute_len() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();
        let call = FunctionCall {
            name: "len".to_string(),
            arguments: vec!["hello".to_string()],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert_eq!(result.output, "5");
    }

    #[test]
    fn test_execute_repeat() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();
        let call = FunctionCall {
            name: "repeat".to_string(),
            arguments: vec!["ab".to_string(), "3".to_string()],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert_eq!(result.output, "ababab");
    }

    #[test]
    fn test_execute_replace() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();
        let call = FunctionCall {
            name: "replace".to_string(),
            arguments: vec![
                "hello world".to_string(),
                "world".to_string(),
                "rust".to_string(),
            ],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert_eq!(result.output, "hello rust");
    }

    #[test]
    fn test_execute_uuid() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();
        let call = FunctionCall {
            name: "uuid".to_string(),
            arguments: vec![],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert!(Uuid::parse_str(&result.output).is_ok());
    }

    #[test]
    fn test_execute_var() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();
        let call = FunctionCall {
            name: "var".to_string(),
            arguments: vec!["name".to_string()],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert_eq!(result.output, "World");
    }

    #[test]
    fn test_execute_unknown_function() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();
        let call = FunctionCall {
            name: "nonexistent".to_string(),
            arguments: vec![],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert!(result.output.contains("unknown function"));
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_expand_document() {
        let registry = FunctionRegistry::new();
        let ctx = make_ctx();

        let content = "Hello .var {name}! Sum is .sum {1} {2} {3}.";
        let result = registry.expand_document(content, &ctx);

        assert!(result.contains("Hello World!"));
        assert!(result.contains("Sum is 6."));
    }

    #[test]
    fn test_generate_toc() {
        let registry = FunctionRegistry::new();
        let content = "# Introduction\n## Getting Started\n### Installation\n## Usage";

        let toc = registry.generate_toc(content);

        assert!(toc.contains("- [Introduction](#introduction)"));
        assert!(toc.contains("  - [Getting Started](#getting-started)"));
        assert!(toc.contains("    - [Installation](#installation)"));
    }

    #[test]
    fn test_template_function() {
        let mut registry = FunctionRegistry::new();

        let template_fn = FunctionDefinition {
            id: Uuid::new_v4(),
            name: "greet".to_string(),
            parameters: vec![param("name", ParameterType::String, true)],
            body: FunctionBody::Template("Hello, {{0}}!".to_string()),
            description: Some("Greet someone".to_string()),
            created_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        registry.register(template_fn);

        let ctx = make_ctx();
        let call = FunctionCall {
            name: "greet".to_string(),
            arguments: vec!["Alice".to_string()],
            start_pos: 0,
            end_pos: 0,
            raw_text: String::new(),
        };

        let result = registry.execute(&call, &ctx).unwrap();
        assert_eq!(result.output, "Hello, Alice!");
    }

    #[test]
    fn test_list_functions() {
        let registry = FunctionRegistry::new();
        let names = registry.list();

        assert!(names.contains(&"date"));
        assert!(names.contains(&"sum"));
        assert!(names.contains(&"upper"));
        assert!(names.len() >= 20); // We registered 20+ built-ins
    }

    #[test]
    fn test_format_number_integer() {
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(0.0), "0");
    }

    #[test]
    fn test_format_number_float() {
        assert_eq!(format_number(3.14), "3.14");
    }
}
