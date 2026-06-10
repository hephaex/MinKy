//! Helpers for handling reasoning-model output.
//!
//! Reasoning models (e.g. Upstage `solar-open2`) wrap their chain-of-thought in
//! `<think>...</think>` before the actual answer. [`strip_reasoning`] removes
//! that so downstream parsers see only the final answer. It is a no-op on plain
//! output from non-reasoning models.

/// Models that wrap answers in reasoning output and need stripping.
pub const REASONING_MODELS: &[&str] = &["solar-open2-260528"];

/// Return true when the model wraps answers in `<think>` reasoning output.
pub fn is_reasoning_model(model: &str) -> bool {
    REASONING_MODELS.contains(&model)
}

/// Remove a reasoning model's `<think>` output, returning the final answer.
///
/// Closed `<think>...</think>` blocks are removed; an unclosed `<think>` tail
/// (token budget exhausted mid-thought) is dropped. Plain text without `<think>`
/// is returned trimmed.
pub fn strip_reasoning(text: &str) -> String {
    const OPEN: &str = "<think>";
    const CLOSE: &str = "</think>";

    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    loop {
        match rest.find(OPEN) {
            None => {
                out.push_str(rest);
                break;
            }
            Some(open) => {
                out.push_str(&rest[..open]);
                let after_open = &rest[open + OPEN.len()..];
                match after_open.find(CLOSE) {
                    Some(close) => {
                        rest = &after_open[close + CLOSE.len()..];
                    }
                    None => {
                        // Unclosed <think>: discard the remaining tail.
                        break;
                    }
                }
            }
        }
    }

    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_closed_think_block() {
        assert_eq!(strip_reasoning("<think>weighing</think>q1\nq2"), "q1\nq2");
    }

    #[test]
    fn removes_multiline_think_block() {
        let text = "<think>\nline1\nline2\n</think>\nalt query";
        assert_eq!(strip_reasoning(text), "alt query");
    }

    #[test]
    fn drops_unclosed_think_tail() {
        assert_eq!(strip_reasoning("answer<think>truncated thought"), "answer");
    }

    #[test]
    fn plain_output_unchanged() {
        assert_eq!(strip_reasoning("alt one\nalt two"), "alt one\nalt two");
    }

    #[test]
    fn removes_multiple_blocks() {
        assert_eq!(strip_reasoning("<think>a</think>x<think>b</think>y"), "xy");
    }

    #[test]
    fn detects_reasoning_models() {
        assert!(is_reasoning_model("solar-open2-260528"));
        assert!(!is_reasoning_model("claude-haiku-4-5-20251101"));
    }
}
