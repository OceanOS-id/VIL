//! # vil_rules — VIL Business Rule Engine
//!
//! YAML-based if/then rules with VIL Expression compatible conditions (via vil_expr).
//! Supports two formats: condition rules and decision tables.
//!
//! ## Condition Rules
//! ```yaml
//! id: credit_scoring_v1
//! rules:
//!   - id: high_risk
//!     condition: "score < 500 && outstanding > 100000000"
//!     action: { risk_level: "high", max_credit: 0 }
//!   - id: low_risk
//!     condition: "score >= 700"
//!     action: { risk_level: "low", max_credit: 500000000 }
//! ```
//!
//! ## Decision Table
//! ```yaml
//! id: pricing_v1
//! type: decision_table
//! rules:
//!   - when: { tier: "enterprise", total: ">1000000" }
//!     then: { discount: 20 }
//!   - when: { tier: "starter" }
//!     then: { discount: 0 }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("parse: {0}")]
    Parse(String),
    #[error("eval: {0}")]
    Eval(String),
}

// ── Rule Set ──

#[derive(Debug, Clone, Deserialize)]
pub struct RuleSet {
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub rule_type: Option<String>,
    pub rules: Vec<RuleDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuleDef {
    pub id: Option<String>,
    // Condition rules
    pub condition: Option<String>,
    pub action: Option<Value>,
    // Decision table
    pub when: Option<HashMap<String, Value>>,
    pub then: Option<Value>,
}

// ── Result ──

#[derive(Debug, Clone, Serialize)]
pub struct RuleResult {
    pub matched: Vec<RuleMatch>,
    pub first_action: Option<Value>,
    pub all_actions: Vec<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleMatch {
    pub rule_id: String,
    pub action: Value,
}

// ── API ──

impl RuleSet {
    /// Load from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, RuleError> {
        serde_yaml::from_str(yaml).map_err(|e| RuleError::Parse(e.to_string()))
    }

    /// Load from YAML file.
    pub fn from_file(path: &str) -> Result<Self, RuleError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| RuleError::Parse(format!("read {}: {}", path, e)))?;
        Self::from_yaml(&content)
    }

    /// Evaluate all rules against input data. Returns all matches.
    pub fn evaluate(&self, input: &Value) -> Result<RuleResult, RuleError> {
        let vars = value_to_vars(input);
        let is_decision_table = self.rule_type.as_deref() == Some("decision_table");

        let mut matched = Vec::new();

        for (idx, rule) in self.rules.iter().enumerate() {
            let rule_id = rule.id.clone().unwrap_or_else(|| format!("rule_{}", idx));

            let matches = if is_decision_table {
                self.eval_decision_row(rule, &vars)?
            } else {
                self.eval_condition_rule(rule, &vars)?
            };

            if matches {
                let action = if is_decision_table {
                    rule.then.clone().unwrap_or(Value::Null)
                } else {
                    rule.action.clone().unwrap_or(Value::Null)
                };
                matched.push(RuleMatch { rule_id, action });
            }
        }

        let first_action = matched.first().map(|m| m.action.clone());
        let all_actions = matched.iter().map(|m| m.action.clone()).collect();

        Ok(RuleResult { matched, first_action, all_actions })
    }

    fn eval_condition_rule(&self, rule: &RuleDef, vars: &vil_expr::Vars) -> Result<bool, RuleError> {
        match &rule.condition {
            Some(cond) => vil_expr::evaluate_bool(cond, vars)
                .map_err(|e| RuleError::Eval(format!("rule {}: {}", rule.id.as_deref().unwrap_or("?"), e))),
            None => Ok(true), // no condition = always match (catch-all)
        }
    }

    fn eval_decision_row(&self, rule: &RuleDef, vars: &vil_expr::Vars) -> Result<bool, RuleError> {
        let when = match &rule.when {
            Some(w) => w,
            None => return Ok(true), // no when = always match
        };

        for (field, expected) in when {
            let actual = vars.get(field).cloned().unwrap_or(Value::Null);

            let matches = match expected {
                // String with operator prefix: ">100", ">=500", "!=0"
                Value::String(s) if s.starts_with('>') || s.starts_with('<') || s.starts_with('!') => {
                    let expr = format!("{} {}", field, s);
                    vil_expr::evaluate_bool(&expr, vars)
                        .map_err(|e| RuleError::Eval(e))?
                }
                // Exact match
                _ => actual == *expected,
            };

            if !matches { return Ok(false); }
        }

        Ok(true) // all fields matched
    }
}

/// Convert a JSON Value (object or flat) into vil_expr Vars.
fn value_to_vars(input: &Value) -> vil_expr::Vars {
    let mut vars = HashMap::new();
    match input {
        Value::Object(map) => {
            for (k, v) in map {
                vars.insert(k.clone(), v.clone());
            }
        }
        _ => {
            vars.insert("_input".into(), input.clone());
        }
    }
    vars
}

/// Convenience: evaluate rules from YAML string against input.
pub fn evaluate_rules(rules_yaml: &str, input: &Value) -> Result<RuleResult, RuleError> {
    let rs = RuleSet::from_yaml(rules_yaml)?;
    rs.evaluate(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const CREDIT_RULES: &str = r#"
id: credit_scoring_v1
rules:
  - id: high_risk
    condition: "score < 500 && outstanding > 100000000"
    action: { risk_level: "high", max_credit: 0, recommendation: "reject" }
  - id: medium_risk
    condition: "score >= 500 && score < 700"
    action: { risk_level: "medium", max_credit: 50000000 }
  - id: low_risk
    condition: "score >= 700"
    action: { risk_level: "low", max_credit: 500000000, recommendation: "approve" }
  - id: blacklisted
    condition: "blacklist == true"
    action: { risk_level: "rejected", max_credit: 0 }
"#;

    #[test]
    fn test_high_risk() {
        let input = json!({"score": 400, "outstanding": 200000000, "blacklist": false});
        let result = evaluate_rules(CREDIT_RULES, &input).unwrap();
        assert_eq!(result.first_action.unwrap()["risk_level"], "high");
        assert_eq!(result.matched.len(), 1);
    }

    #[test]
    fn test_medium_risk() {
        let input = json!({"score": 600, "outstanding": 50000000, "blacklist": false});
        let result = evaluate_rules(CREDIT_RULES, &input).unwrap();
        assert_eq!(result.first_action.unwrap()["risk_level"], "medium");
    }

    #[test]
    fn test_low_risk() {
        let input = json!({"score": 800, "outstanding": 10000000, "blacklist": false});
        let result = evaluate_rules(CREDIT_RULES, &input).unwrap();
        assert_eq!(result.first_action.unwrap()["risk_level"], "low");
    }

    #[test]
    fn test_blacklisted() {
        let input = json!({"score": 800, "outstanding": 0, "blacklist": true});
        let result = evaluate_rules(CREDIT_RULES, &input).unwrap();
        // Both low_risk and blacklisted match
        assert!(result.matched.len() >= 2);
        // Check blacklisted is in results
        assert!(result.matched.iter().any(|m| m.rule_id == "blacklisted"));
    }

    #[test]
    fn test_multiple_matches() {
        let input = json!({"score": 800, "outstanding": 0, "blacklist": true});
        let result = evaluate_rules(CREDIT_RULES, &input).unwrap();
        assert!(result.all_actions.len() >= 2);
    }

    const PRICING_TABLE: &str = r#"
id: pricing_v1
type: decision_table
rules:
  - when: { tier: "enterprise" }
    then: { discount: 20, free_shipping: true }
  - when: { tier: "pro" }
    then: { discount: 10, free_shipping: true }
  - when: { tier: "starter" }
    then: { discount: 0, free_shipping: false }
"#;

    #[test]
    fn test_decision_table_enterprise() {
        let input = json!({"tier": "enterprise", "total": 2000000});
        let result = evaluate_rules(PRICING_TABLE, &input).unwrap();
        assert_eq!(result.first_action.unwrap()["discount"], 20);
    }

    #[test]
    fn test_decision_table_starter() {
        let input = json!({"tier": "starter", "total": 50000});
        let result = evaluate_rules(PRICING_TABLE, &input).unwrap();
        assert_eq!(result.first_action.unwrap()["discount"], 0);
    }

    #[test]
    fn test_decision_table_no_match() {
        let input = json!({"tier": "unknown"});
        let result = evaluate_rules(PRICING_TABLE, &input).unwrap();
        assert!(result.matched.is_empty());
    }

    const OPERATOR_TABLE: &str = r#"
id: threshold_v1
type: decision_table
rules:
  - when: { score: ">80" }
    then: { grade: "A" }
  - when: { score: ">=60" }
    then: { grade: "B" }
"#;

    #[test]
    fn test_decision_table_operators() {
        let input = json!({"score": 85});
        let result = evaluate_rules(OPERATOR_TABLE, &input).unwrap();
        assert_eq!(result.first_action.unwrap()["grade"], "A");
    }

    #[test]
    fn test_from_yaml() {
        let rs = RuleSet::from_yaml(CREDIT_RULES).unwrap();
        assert_eq!(rs.id, "credit_scoring_v1");
        assert_eq!(rs.rules.len(), 4);
    }
}
