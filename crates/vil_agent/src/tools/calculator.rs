//! Simple math calculator tool for the agent.

use async_trait::async_trait;
use crate::tool::{Tool, ToolError, ToolResult};

/// Tool that evaluates basic mathematical expressions.
pub struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "Evaluate a mathematical expression"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "Math expression (e.g., '2 + 3 * 4')"
                }
            },
            "required": ["expression"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let expr = params["expression"]
            .as_str()
            .ok_or(ToolError::InvalidParameters("missing expression".into()))?;
        let result = simple_eval(expr).map_err(ToolError::ExecutionFailed)?;
        Ok(ToolResult {
            output: result.to_string(),
            metadata: None,
        })
    }
}

/// Evaluate a basic arithmetic expression supporting +, -, *, /.
///
/// Handles operator precedence by parsing + and - at the top level first
/// (rightmost split), then * and / within those terms.
fn simple_eval(expr: &str) -> Result<f64, String> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err("empty expression".into());
    }

    // Try parsing as a single number first
    if let Ok(n) = expr.parse::<f64>() {
        return Ok(n);
    }

    // Handle parentheses stripped
    if expr.starts_with('(') && expr.ends_with(')') {
        let inner = &expr[1..expr.len() - 1];
        // Check balanced parens
        let mut depth = 0i32;
        let mut balanced = true;
        for ch in inner.chars() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth < 0 {
                        balanced = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if balanced && depth == 0 {
            return simple_eval(inner);
        }
    }

    // Find the rightmost + or - at depth 0 (lowest precedence)
    let mut depth = 0i32;
    let mut split_pos: Option<usize> = None;
    let mut split_op: Option<char> = None;

    for (i, ch) in expr.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            '+' | '-' if depth == 0 && i > 0 => {
                split_pos = Some(i);
                split_op = Some(ch);
            }
            _ => {}
        }
    }

    if let (Some(pos), Some(op)) = (split_pos, split_op) {
        let left = simple_eval(&expr[..pos])?;
        let right = simple_eval(&expr[pos + 1..])?;
        return match op {
            '+' => Ok(left + right),
            '-' => Ok(left - right),
            _ => unreachable!(),
        };
    }

    // Find the rightmost * or / at depth 0
    depth = 0;
    split_pos = None;
    split_op = None;

    for (i, ch) in expr.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            '*' | '/' if depth == 0 && i > 0 => {
                split_pos = Some(i);
                split_op = Some(ch);
            }
            _ => {}
        }
    }

    if let (Some(pos), Some(op)) = (split_pos, split_op) {
        let left = simple_eval(&expr[..pos])?;
        let right = simple_eval(&expr[pos + 1..])?;
        return match op {
            '*' => Ok(left * right),
            '/' => {
                if right == 0.0 {
                    Err("division by zero".into())
                } else {
                    Ok(left / right)
                }
            }
            _ => unreachable!(),
        };
    }

    Err(format!("cannot parse: {}", expr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_number() {
        assert_eq!(simple_eval("42").unwrap(), 42.0);
        assert_eq!(simple_eval(" 3.14 ").unwrap(), 3.14);
    }

    #[test]
    fn test_addition() {
        assert_eq!(simple_eval("2+3").unwrap(), 5.0);
        assert_eq!(simple_eval("10 + 20").unwrap(), 30.0);
    }

    #[test]
    fn test_subtraction() {
        assert_eq!(simple_eval("10-3").unwrap(), 7.0);
    }

    #[test]
    fn test_multiplication() {
        assert_eq!(simple_eval("4*5").unwrap(), 20.0);
    }

    #[test]
    fn test_division() {
        assert_eq!(simple_eval("10/2").unwrap(), 5.0);
    }

    #[test]
    fn test_division_by_zero() {
        assert!(simple_eval("10/0").is_err());
    }

    #[test]
    fn test_operator_precedence() {
        // 2 + 3 * 4 = 14 (not 20)
        assert_eq!(simple_eval("2+3*4").unwrap(), 14.0);
    }

    #[test]
    fn test_complex_expression() {
        // 10 - 2 * 3 = 4
        assert_eq!(simple_eval("10-2*3").unwrap(), 4.0);
    }

    #[test]
    fn test_empty_expression() {
        assert!(simple_eval("").is_err());
    }

    #[tokio::test]
    async fn test_calculator_tool_execute() {
        let tool = CalculatorTool;
        let result = tool
            .execute(serde_json::json!({"expression": "2+3"}))
            .await
            .unwrap();
        assert_eq!(result.output, "5");
    }

    #[tokio::test]
    async fn test_calculator_tool_missing_param() {
        let tool = CalculatorTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }
}
