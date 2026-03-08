use regex::Regex;

use crate::models::{FunctionCall, FunctionContext, FunctionResult};

/// Parser for Quarkdown-style function calls
/// Syntax: .functionName {arg1} {arg2} ...
pub struct FunctionParser {
    /// Regex for matching function calls: .name {arg}*
    function_regex: Regex,
    /// Regex for matching control structures
    if_regex: Regex,
    foreach_regex: Regex,
}

impl Default for FunctionParser {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionParser {
    pub fn new() -> Self {
        Self {
            // Match .functionName optionally followed by {arg} blocks
            // Examples: .date, .date {%Y-%m-%d}, .sum {1} {2} {3}
            function_regex: Regex::new(r"\.([a-zA-Z_][a-zA-Z0-9_]*)(?:\s*\{([^}]*)\})*").unwrap(),
            // Match .if {condition} ... .endif
            if_regex: Regex::new(r"\.if\s*\{([^}]*)\}([\s\S]*?)\.endif").unwrap(),
            // Match .foreach {item} in {list} ... .endforeach
            foreach_regex: Regex::new(
                r"\.foreach\s*\{([^}]*)\}\s*in\s*\{([^}]*)\}([\s\S]*?)\.endforeach",
            )
            .unwrap(),
        }
    }

    /// Parse text and extract all function calls
    pub fn parse(&self, text: &str) -> Vec<FunctionCall> {
        let mut calls = Vec::new();

        // First, find all simple function calls
        for cap in self.function_regex.captures_iter(text) {
            let full_match = cap.get(0).unwrap();
            let name = cap.get(1).unwrap().as_str().to_string();

            // Extract all arguments from {arg} blocks
            let arguments = self.extract_arguments(full_match.as_str());

            calls.push(FunctionCall {
                name,
                arguments,
                start_pos: full_match.start(),
                end_pos: full_match.end(),
                raw_text: full_match.as_str().to_string(),
            });
        }

        calls
    }

    /// Extract arguments from a function call string
    fn extract_arguments(&self, text: &str) -> Vec<String> {
        let arg_regex = Regex::new(r"\{([^}]*)\}").unwrap();
        arg_regex
            .captures_iter(text)
            .map(|cap| cap.get(1).unwrap().as_str().to_string())
            .collect()
    }

    /// Parse and handle control structures (if/foreach)
    pub fn parse_control_structures(
        &self,
        text: &str,
        ctx: &FunctionContext,
    ) -> (String, Vec<FunctionCall>) {
        let mut result = text.to_string();
        let mut calls = Vec::new();

        // Process .if ... .endif blocks
        result = self.process_if_blocks(&result, ctx);

        // Process .foreach ... .endforeach blocks
        result = self.process_foreach_blocks(&result, ctx);

        // Parse remaining function calls
        calls.extend(self.parse(&result));

        (result, calls)
    }

    /// Process .if {condition} ... .endif blocks
    fn process_if_blocks(&self, text: &str, ctx: &FunctionContext) -> String {
        let mut result = text.to_string();

        while let Some(cap) = self.if_regex.captures(&result) {
            let full_match = cap.get(0).unwrap();
            let condition = cap.get(1).unwrap().as_str().trim();
            let body = cap.get(2).unwrap().as_str();

            let evaluated = self.evaluate_condition(condition, ctx);

            let replacement = if evaluated {
                body.trim().to_string()
            } else {
                String::new()
            };

            result = format!(
                "{}{}{}",
                &result[..full_match.start()],
                replacement,
                &result[full_match.end()..]
            );
        }

        result
    }

    /// Process .foreach {item} in {list} ... .endforeach blocks
    fn process_foreach_blocks(&self, text: &str, ctx: &FunctionContext) -> String {
        let mut result = text.to_string();

        while let Some(cap) = self.foreach_regex.captures(&result) {
            let full_match = cap.get(0).unwrap();
            let item_var = cap.get(1).unwrap().as_str().trim();
            let list_expr = cap.get(2).unwrap().as_str().trim();
            let body = cap.get(3).unwrap().as_str();

            // Parse list expression (comma-separated or variable reference)
            let items = self.parse_list(list_expr, ctx);

            let mut expansion = String::new();
            for item in items {
                let item_body = body.replace(&format!(".var {{{}}}", item_var), &item);
                expansion.push_str(item_body.trim());
                expansion.push('\n');
            }

            result = format!(
                "{}{}{}",
                &result[..full_match.start()],
                expansion.trim(),
                &result[full_match.end()..]
            );
        }

        result
    }

    /// Evaluate a condition expression
    fn evaluate_condition(&self, condition: &str, ctx: &FunctionContext) -> bool {
        // Simple conditions:
        // - Variable existence: varname
        // - Equality: a == b
        // - Not empty: !empty(varname)

        let condition = condition.trim();

        // Check for equality
        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").collect();
            if parts.len() == 2 {
                let left = self.resolve_value(parts[0].trim(), ctx);
                let right = self.resolve_value(parts[1].trim(), ctx);
                return left == right;
            }
        }

        // Check for inequality
        if condition.contains("!=") {
            let parts: Vec<&str> = condition.split("!=").collect();
            if parts.len() == 2 {
                let left = self.resolve_value(parts[0].trim(), ctx);
                let right = self.resolve_value(parts[1].trim(), ctx);
                return left != right;
            }
        }

        // Check for not empty
        if condition.starts_with("!empty(") && condition.ends_with(')') {
            let var_name = &condition[7..condition.len() - 1];
            if let Some(value) = ctx.variables.get(var_name) {
                return !value.is_empty();
            }
            return false;
        }

        // Check for empty
        if condition.starts_with("empty(") && condition.ends_with(')') {
            let var_name = &condition[6..condition.len() - 1];
            if let Some(value) = ctx.variables.get(var_name) {
                return value.is_empty();
            }
            return true;
        }

        // Check variable existence (truthy)
        if let Some(value) = ctx.variables.get(condition) {
            return !value.is_empty() && value != "false" && value != "0";
        }

        // Literal boolean
        condition == "true"
    }

    /// Resolve a value (variable or literal)
    fn resolve_value(&self, value: &str, ctx: &FunctionContext) -> String {
        let value = value.trim();

        // String literal
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            return value[1..value.len() - 1].to_string();
        }

        // Variable reference
        if let Some(var_value) = ctx.variables.get(value) {
            return var_value.clone();
        }

        // Return as literal
        value.to_string()
    }

    /// Parse a list expression
    fn parse_list(&self, expr: &str, ctx: &FunctionContext) -> Vec<String> {
        // Check if it's a variable reference
        if let Some(value) = ctx.variables.get(expr) {
            return value.split(',').map(|s| s.trim().to_string()).collect();
        }

        // Parse as comma-separated list
        expr.split(',').map(|s| s.trim().to_string()).collect()
    }

    /// Replace variables in text
    pub fn replace_variables(&self, text: &str, ctx: &FunctionContext) -> String {
        let var_regex = Regex::new(r"\.var\s*\{([^}]*)\}").unwrap();
        let mut result = text.to_string();

        while let Some(cap) = var_regex.captures(&result) {
            let full_match = cap.get(0).unwrap();
            let var_name = cap.get(1).unwrap().as_str().trim();

            let replacement = ctx
                .variables
                .get(var_name)
                .cloned()
                .unwrap_or_else(|| format!("[undefined: {}]", var_name));

            result = format!(
                "{}{}{}",
                &result[..full_match.start()],
                replacement,
                &result[full_match.end()..]
            );
        }

        result
    }

    /// Expand all functions in text using a resolver function
    pub fn expand<F>(&self, text: &str, ctx: &FunctionContext, resolver: F) -> String
    where
        F: Fn(&FunctionCall, &FunctionContext) -> FunctionResult,
    {
        // First process control structures
        let (processed, _calls) = self.parse_control_structures(text, ctx);

        // Replace variables
        let with_vars = self.replace_variables(&processed, ctx);

        // Parse and expand function calls
        let calls = self.parse(&with_vars);

        // Sort calls by position (reverse to maintain positions during replacement)
        let mut sorted_calls = calls;
        sorted_calls.sort_by(|a, b| b.start_pos.cmp(&a.start_pos));

        let mut result = with_vars;

        for call in sorted_calls {
            // Skip control structure functions (already processed)
            if ["if", "endif", "foreach", "endforeach", "var"].contains(&call.name.as_str()) {
                continue;
            }

            let func_result = resolver(&call, ctx);

            result = format!(
                "{}{}{}",
                &result[..call.start_pos],
                func_result.output,
                &result[call.end_pos..]
            );
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_ctx() -> FunctionContext {
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "John".to_string());
        variables.insert("title".to_string(), "Document".to_string());
        variables.insert("empty_var".to_string(), String::new());
        variables.insert("items".to_string(), "a, b, c".to_string());

        FunctionContext {
            variables,
            document_id: None,
            base_path: None,
        }
    }

    #[test]
    fn test_parse_simple_function() {
        let parser = FunctionParser::new();
        let calls = parser.parse(".date");

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "date");
        assert!(calls[0].arguments.is_empty());
    }

    #[test]
    fn test_parse_function_with_one_arg() {
        let parser = FunctionParser::new();
        let calls = parser.parse(".date {%Y-%m-%d}");

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "date");
        assert_eq!(calls[0].arguments, vec!["%Y-%m-%d"]);
    }

    #[test]
    fn test_parse_function_with_multiple_args() {
        let parser = FunctionParser::new();
        let calls = parser.parse(".sum {1} {2} {3}");

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "sum");
        assert_eq!(calls[0].arguments, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_parse_multiple_functions() {
        let parser = FunctionParser::new();
        let text = "Today is .date {%Y-%m-%d} and total is .sum {10} {20}";
        let calls = parser.parse(text);

        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "date");
        assert_eq!(calls[1].name, "sum");
    }

    #[test]
    fn test_replace_variables() {
        let parser = FunctionParser::new();
        let ctx = make_ctx();

        let result = parser.replace_variables("Hello .var {name}!", &ctx);
        assert_eq!(result, "Hello John!");
    }

    #[test]
    fn test_replace_undefined_variable() {
        let parser = FunctionParser::new();
        let ctx = make_ctx();

        let result = parser.replace_variables("Hello .var {unknown}!", &ctx);
        assert_eq!(result, "Hello [undefined: unknown]!");
    }

    #[test]
    fn test_evaluate_condition_truthy() {
        let parser = FunctionParser::new();
        let ctx = make_ctx();

        assert!(parser.evaluate_condition("name", &ctx));
        assert!(parser.evaluate_condition("true", &ctx));
    }

    #[test]
    fn test_evaluate_condition_falsy() {
        let parser = FunctionParser::new();
        let ctx = make_ctx();

        assert!(!parser.evaluate_condition("unknown_var", &ctx));
        assert!(!parser.evaluate_condition("false", &ctx));
    }

    #[test]
    fn test_evaluate_condition_equality() {
        let parser = FunctionParser::new();
        let ctx = make_ctx();

        assert!(parser.evaluate_condition("name == \"John\"", &ctx));
        assert!(!parser.evaluate_condition("name == \"Jane\"", &ctx));
    }

    #[test]
    fn test_evaluate_condition_empty() {
        let parser = FunctionParser::new();
        let ctx = make_ctx();

        assert!(parser.evaluate_condition("empty(empty_var)", &ctx));
        assert!(!parser.evaluate_condition("empty(name)", &ctx));
        assert!(parser.evaluate_condition("!empty(name)", &ctx));
    }

    #[test]
    fn test_process_if_block_true() {
        let parser = FunctionParser::new();
        let ctx = make_ctx();

        let text = ".if {name}\nHello World\n.endif";
        let result = parser.process_if_blocks(text, &ctx);

        assert!(result.contains("Hello World"));
        assert!(!result.contains(".if"));
        assert!(!result.contains(".endif"));
    }

    #[test]
    fn test_process_if_block_false() {
        let parser = FunctionParser::new();
        let ctx = make_ctx();

        let text = ".if {unknown_var}\nHello World\n.endif";
        let result = parser.process_if_blocks(text, &ctx);

        assert!(!result.contains("Hello World"));
    }

    #[test]
    fn test_process_foreach_block() {
        let parser = FunctionParser::new();
        let ctx = make_ctx();

        let text = ".foreach {x} in {a, b, c}\nItem: .var {x}\n.endforeach";
        let result = parser.process_foreach_blocks(text, &ctx);

        assert!(result.contains("Item: a"));
        assert!(result.contains("Item: b"));
        assert!(result.contains("Item: c"));
    }

    #[test]
    fn test_expand_with_resolver() {
        let parser = FunctionParser::new();
        let ctx = make_ctx();

        let text = "Today is .date {%Y}";

        let result = parser.expand(text, &ctx, |call, _ctx| {
            if call.name == "date" {
                FunctionResult::ok("2026".to_string())
            } else {
                FunctionResult::ok(String::new())
            }
        });

        assert_eq!(result, "Today is 2026");
    }

    #[test]
    fn test_function_call_positions() {
        let parser = FunctionParser::new();
        let text = "Before .date After";
        let calls = parser.parse(text);

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].start_pos, 7);
        assert_eq!(calls[0].end_pos, 12);
    }

    #[test]
    fn test_underscore_in_function_name() {
        let parser = FunctionParser::new();
        let calls = parser.parse(".my_function {arg}");

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "my_function");
    }

    #[test]
    fn test_number_in_function_name() {
        let parser = FunctionParser::new();
        let calls = parser.parse(".func123 {arg}");

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "func123");
    }
}
