//! # vil_rules — VIL Business Rule Engine (vdicl-compatible)
//!
//! Non-optimized implementation of VFlow's VRule engine.
//! Supports the vdicl (Vastar Decision & Inspection Control Language) format
//! used in production SLIK-AUTOHUB and similar regulatory systems.
//!
//! ## Features
//! - Condition rules with vil_expr expressions
//! - Decision tables with operator predicates
//! - Schema reference validation
//! - Hit policies: COLLECT, FIRST, UNIQUE
//! - Multi-action `then`: EMIT, SET, SET_DECISION, ADD_SCORE, ABORT
//! - Priority ordering with enable/disable toggle
//! - Risk scoring accumulation
//!
//! ## vdicl Format
//! ```yaml
//! metadata:
//!   rulepack_id: "slik_mandatory_fields_v1"
//!   rulepack_name: "SLIK Mandatory Fields"
//! schema_ref:
//!   path: "schemas/slik_submission_fact_v1.yaml"
//! hit_policy: COLLECT
//! rules:
//!   - id: "mandatory-r001"
//!     priority: 10
//!     enabled: true
//!     when: "tenant_id IS NULL OR ISBLANK(tenant_id)"
//!     then:
//!       - kind: EMIT
//!         severity: ERROR
//!         code: TENANT_MISSING
//!         field: tenant_id
//!         msg: "Tenant ID wajib diisi"
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

// ═══════════════════════════════════════════════════════════════════
// Rule Set (vdicl-compatible)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize)]
pub struct RuleSet {
    /// Rule set ID (used as rule_set_id in workflows).
    /// In vdicl: from metadata.rulepack_id or top-level `id`.
    #[serde(default)]
    pub id: String,
    pub name: Option<String>,

    /// vdicl metadata block.
    #[serde(default)]
    pub metadata: Option<Metadata>,

    /// Schema reference for input validation.
    #[serde(default)]
    pub schema_ref: Option<SchemaRef>,

    /// Rule type: null (condition rules) or "decision_table".
    #[serde(rename = "type")]
    pub rule_type: Option<String>,

    /// Hit policy: COLLECT (all matches), FIRST (first match), UNIQUE (exactly one).
    #[serde(default = "default_hit_policy")]
    pub hit_policy: String,

    /// Aggregate function for COLLECT: LIST, SUM, COUNT.
    #[serde(default = "default_aggregate_fn")]
    pub aggregate_fn: String,

    /// Rules list.
    #[serde(default)]
    pub rules: Vec<RuleDef>,
}

fn default_hit_policy() -> String { "COLLECT".into() }
fn default_aggregate_fn() -> String { "LIST".into() }

#[derive(Debug, Clone, Deserialize)]
pub struct Metadata {
    pub project: Option<String>,
    pub author: Option<String>,
    pub environment: Option<String>,
    pub rulepack_name: Option<String>,
    pub rulepack_description: Option<String>,
    pub rulepack_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchemaRef {
    pub path: Option<String>,
    pub namespace: Option<String>,
    pub entity: Option<String>,
    pub version: Option<u32>,
}

// ═══════════════════════════════════════════════════════════════════
// Rule Definition
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize)]
pub struct RuleDef {
    pub id: Option<String>,
    pub description: Option<String>,

    /// Priority (lower = higher priority). Default: 100.
    #[serde(default = "default_priority")]
    pub priority: i32,

    /// Enable/disable toggle. Default: true.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    // ── Condition (legacy) ──
    pub condition: Option<String>,
    /// Single action value (legacy format).
    pub action: Option<Value>,

    // ── `when` — can be a String (vdicl expr) or a Map (decision table) ──
    #[serde(rename = "when", default, deserialize_with = "deserialize_when")]
    pub when_clause: WhenClause,

    // ── `then` — can be a Vec<Action> (vdicl) or a Value (legacy/decision table) ──
    #[serde(rename = "then", default, deserialize_with = "deserialize_then")]
    pub then_clause: ThenClause,
}

/// The `when` field can be a vdicl expression string OR a decision table map.
#[derive(Debug, Clone, Default)]
pub enum WhenClause {
    #[default]
    None,
    Expr(String),
    Table(HashMap<String, Value>),
}

/// The `then` field can be a vdicl multi-action list OR a single value.
#[derive(Debug, Clone, Default)]
pub enum ThenClause {
    #[default]
    None,
    Actions(Vec<Action>),
    Value(Value),
}

fn deserialize_when<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<WhenClause, D::Error> {
    let val = Value::deserialize(deserializer)?;
    match val {
        Value::Null => Ok(WhenClause::None),
        Value::String(s) => Ok(WhenClause::Expr(s)),
        Value::Object(map) => {
            let hm: HashMap<String, Value> = map.into_iter().collect();
            Ok(WhenClause::Table(hm))
        }
        _ => Err(serde::de::Error::custom("'when' must be a string or object")),
    }
}

fn deserialize_then<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<ThenClause, D::Error> {
    let val = Value::deserialize(deserializer)?;
    match &val {
        Value::Null => Ok(ThenClause::None),
        Value::Array(_arr) => {
            // Try parse as Vec<Action>
            match serde_json::from_value::<Vec<Action>>(val.clone()) {
                Ok(actions) => Ok(ThenClause::Actions(actions)),
                Err(_) => Ok(ThenClause::Value(val)),
            }
        }
        _ => Ok(ThenClause::Value(val)),
    }
}

fn default_priority() -> i32 { 100 }
fn default_enabled() -> bool { true }

// ═══════════════════════════════════════════════════════════════════
// Actions (vdicl)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind")]
#[allow(non_camel_case_types)]
pub enum Action {
    /// Emit a finding/validation result.
    EMIT {
        severity: Option<String>,
        code: Option<String>,
        field: Option<String>,
        msg: Option<String>,
    },
    /// Set an output variable at a dotted path.
    SET {
        path: Option<String>,
        value: Option<Value>,
    },
    /// Set the decision outcome (APPROVE, REVIEW, REJECT).
    SET_DECISION {
        decision: Option<String>,
    },
    /// Add to risk score accumulator.
    ADD_SCORE {
        score_delta: Option<i32>,
    },
    /// Abort rule evaluation (stop processing remaining rules).
    ABORT {
        msg: Option<String>,
    },
}

// ═══════════════════════════════════════════════════════════════════
// Evaluation Result
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Default)]
pub struct RuleResult {
    /// Findings from EMIT actions.
    pub findings: Vec<Finding>,
    /// Final decision from SET_DECISION (last one wins).
    pub decision: Option<String>,
    /// Accumulated risk score from ADD_SCORE.
    pub score: i32,
    /// Output variables from SET actions.
    pub outputs: HashMap<String, Value>,
    /// Whether evaluation was aborted.
    pub aborted: bool,
    /// Number of rules evaluated.
    pub rules_evaluated: u32,
    /// Number of rules matched.
    pub rules_matched: u32,

    // Legacy compat
    pub matched: Vec<RuleMatch>,
    pub first_action: Option<Value>,
    pub all_actions: Vec<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule_id: String,
    pub severity: String,
    pub code: String,
    pub field: String,
    pub msg: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleMatch {
    pub rule_id: String,
    pub action: Value,
}

// ═══════════════════════════════════════════════════════════════════
// Schema (Fact Schema)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize)]
pub struct FactSchema {
    pub entity: Option<String>,
    pub namespace: Option<String>,
    pub version: Option<u32>,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldDef {
    pub path: String,
    pub scalar: Option<String>,
    #[serde(default = "default_enabled")]
    pub nullable: bool,
    pub note: Option<String>,
}

impl FactSchema {
    pub fn from_file(path: &str) -> Result<Self, RuleError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| RuleError::Parse(format!("read {}: {}", path, e)))?;
        serde_yaml::from_str(&content).map_err(|e| RuleError::Parse(e.to_string()))
    }
}

// ═══════════════════════════════════════════════════════════════════
// API
// ═══════════════════════════════════════════════════════════════════

impl RuleSet {
    /// Load from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, RuleError> {
        let mut rs: RuleSet = serde_yaml::from_str(yaml)
            .map_err(|e| RuleError::Parse(e.to_string()))?;
        // Resolve id from metadata.rulepack_id if top-level id is empty
        if rs.id.is_empty() {
            if let Some(ref meta) = rs.metadata {
                if let Some(ref rp_id) = meta.rulepack_id {
                    rs.id = rp_id.clone();
                }
            }
        }
        Ok(rs)
    }

    /// Load from YAML file.
    pub fn from_file(path: &str) -> Result<Self, RuleError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| RuleError::Parse(format!("read {}: {}", path, e)))?;
        Self::from_yaml(&content)
    }

    /// Evaluate all rules against input data.
    pub fn evaluate(&self, input: &Value) -> Result<RuleResult, RuleError> {
        let vars = value_to_vars(input);
        let is_decision_table = self.rule_type.as_deref() == Some("decision_table");

        // Sort rules by priority
        let mut sorted_rules: Vec<&RuleDef> = self.rules.iter()
            .filter(|r| r.enabled)
            .collect();
        sorted_rules.sort_by_key(|r| r.priority);

        let mut result = RuleResult::default();

        for rule in sorted_rules {
            result.rules_evaluated += 1;
            let rule_id = rule.id.clone().unwrap_or_else(|| format!("rule_{}", result.rules_evaluated));

            // Evaluate condition
            let matches = if is_decision_table {
                self.eval_decision_row(rule, &vars)?
            } else {
                self.eval_condition(rule, &vars)?
            };

            if !matches { continue; }
            result.rules_matched += 1;

            // ── Process actions ──
            match &rule.then_clause {
                ThenClause::Actions(actions) => {
                    for action in actions {
                        match action {
                            Action::EMIT { severity, code, field, msg } => {
                                result.findings.push(Finding {
                                    rule_id: rule_id.clone(),
                                    severity: severity.clone().unwrap_or_else(|| "ERROR".into()),
                                    code: code.clone().unwrap_or_default(),
                                    field: field.clone().unwrap_or_default(),
                                    msg: msg.clone().unwrap_or_default(),
                                });
                            }
                            Action::SET { path, value } => {
                                if let (Some(p), Some(v)) = (path, value) {
                                    result.outputs.insert(p.clone(), v.clone());
                                }
                            }
                            Action::SET_DECISION { decision } => {
                                result.decision = decision.clone();
                            }
                            Action::ADD_SCORE { score_delta } => {
                                result.score += score_delta.unwrap_or(0);
                            }
                            Action::ABORT { msg: _ } => {
                                result.aborted = true;
                            }
                        }
                    }
                }
                ThenClause::Value(v) if !v.is_null() => {
                    result.matched.push(RuleMatch {
                        rule_id: rule_id.clone(),
                        action: v.clone(),
                    });
                }
                _ => {}
            }

            // Legacy: single action field
            if let Some(ref action) = rule.action {
                if !action.is_null() {
                    result.matched.push(RuleMatch {
                        rule_id: rule_id.clone(),
                        action: action.clone(),
                    });
                }
            }

            // Hit policy: FIRST = stop after first match
            if self.hit_policy == "FIRST" { break; }
            // ABORT action stops evaluation
            if result.aborted { break; }
        }

        result.first_action = result.matched.first().map(|m| m.action.clone());
        result.all_actions = result.matched.iter().map(|m| m.action.clone()).collect();

        Ok(result)
    }

    fn eval_condition(&self, rule: &RuleDef, vars: &vil_expr::Vars) -> Result<bool, RuleError> {
        // Check WhenClause first, fallback to legacy `condition`
        match &rule.when_clause {
            WhenClause::Expr(expr) => vil_expr::evaluate_bool(expr, vars)
                .map_err(|e| RuleError::Eval(format!("rule {}: {}", rule.id.as_deref().unwrap_or("?"), e))),
            WhenClause::None => {
                // Fallback to legacy `condition`
                match &rule.condition {
                    Some(cond) => vil_expr::evaluate_bool(cond, vars)
                        .map_err(|e| RuleError::Eval(format!("rule {}: {}", rule.id.as_deref().unwrap_or("?"), e))),
                    None => Ok(true),
                }
            }
            WhenClause::Table(_) => Ok(true), // table handled by eval_decision_row
        }
    }

    fn eval_decision_row(&self, rule: &RuleDef, vars: &vil_expr::Vars) -> Result<bool, RuleError> {
        let when = match &rule.when_clause {
            WhenClause::Table(w) => w,
            _ => return Ok(true),
        };

        for (field, expected) in when {
            let actual = vars.get(field).cloned().unwrap_or(Value::Null);

            let matches = match expected {
                Value::String(s) if s.starts_with('>') || s.starts_with('<') || s.starts_with('!') => {
                    let expr = format!("{} {}", field, s);
                    vil_expr::evaluate_bool(&expr, vars)
                        .map_err(|e| RuleError::Eval(e))?
                }
                _ => val_eq(&actual, expected),
            };

            if !matches { return Ok(false); }
        }

        Ok(true)
    }
}

// ═══════════════════════════════════════════════════════════════════
// Convenience API
// ═══════════════════════════════════════════════════════════════════

/// Evaluate rules from YAML string against input.
pub fn evaluate_rules(rules_yaml: &str, input: &Value) -> Result<RuleResult, RuleError> {
    let rs = RuleSet::from_yaml(rules_yaml)?;
    rs.evaluate(input)
}

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

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

fn val_eq(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (a.as_f64(), b.as_f64()) {
        return x == y;
    }
    a == b
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── Legacy condition rules ──

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
    fn test_low_risk() {
        let input = json!({"score": 800, "outstanding": 10000000, "blacklist": false});
        let result = evaluate_rules(CREDIT_RULES, &input).unwrap();
        assert_eq!(result.first_action.unwrap()["risk_level"], "low");
    }

    #[test]
    fn test_blacklisted() {
        let input = json!({"score": 800, "outstanding": 0, "blacklist": true});
        let result = evaluate_rules(CREDIT_RULES, &input).unwrap();
        assert!(result.matched.iter().any(|m| m.rule_id == "blacklisted"));
    }

    // ── Legacy decision table ──

    const PRICING_TABLE: &str = r#"
id: pricing_v1
type: decision_table
rules:
  - when: { tier: "enterprise" }
    then: { discount: 20, free_shipping: true }
  - when: { tier: "starter" }
    then: { discount: 0, free_shipping: false }
"#;

    #[test]
    fn test_decision_table() {
        let input = json!({"tier": "enterprise", "total": 2000000});
        let result = evaluate_rules(PRICING_TABLE, &input).unwrap();
        assert_eq!(result.first_action.unwrap()["discount"], 20);
    }

    // ── vdicl format (SLIK-style) ──

    const VDICL_MANDATORY: &str = r#"
metadata:
  rulepack_id: "slik_mandatory_fields_v1"
  rulepack_name: "SLIK Mandatory Fields"
schema_ref:
  path: "schemas/slik_submission_fact_v1.yaml"
hit_policy: COLLECT
rules:
  - id: "mandatory-r001"
    priority: 10
    enabled: true
    description: "Tenant ID wajib terisi"
    when: "tenant_id IS NULL OR ISBLANK(tenant_id)"
    then:
      - kind: EMIT
        severity: ERROR
        code: TENANT_MISSING
        field: tenant_id
        msg: "Tenant ID wajib diisi"
      - kind: ABORT
        msg: "Cannot process without tenant_id"

  - id: "mandatory-r002"
    priority: 20
    enabled: true
    when: "nasabah.nik IS NULL OR ISBLANK(nasabah.nik)"
    then:
      - kind: EMIT
        severity: ERROR
        code: NIK_MISSING
        field: nasabah.nik
        msg: "NIK wajib diisi"
"#;

    #[test]
    fn test_vdicl_load() {
        let rs = RuleSet::from_yaml(VDICL_MANDATORY).unwrap();
        assert_eq!(rs.id, "slik_mandatory_fields_v1");
        assert_eq!(rs.rules.len(), 2);
        assert_eq!(rs.hit_policy, "COLLECT");
    }

    #[test]
    fn test_vdicl_emit_and_abort() {
        let input = json!({"tenant_id": null, "nasabah": {"nik": "1234567890123456"}});
        let result = evaluate_rules(VDICL_MANDATORY, &input).unwrap();
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].code, "TENANT_MISSING");
        assert_eq!(result.findings[0].severity, "ERROR");
        assert!(result.aborted); // ABORT stops after first rule
        assert_eq!(result.rules_matched, 1);
    }

    #[test]
    fn test_vdicl_no_findings() {
        let input = json!({"tenant_id": "T001", "nasabah": {"nik": "1234567890123456"}});
        let result = evaluate_rules(VDICL_MANDATORY, &input).unwrap();
        assert!(result.findings.is_empty());
        assert!(!result.aborted);
        assert_eq!(result.rules_evaluated, 2);
        assert_eq!(result.rules_matched, 0);
    }

    // ── vdicl: risk scoring ──

    const VDICL_RISK: &str = r#"
metadata:
  rulepack_id: "risk_scoring_v1"
hit_policy: COLLECT
rules:
  - id: "risk-r001"
    priority: 10
    when: "fasilitas.plafon > 5000000000m"
    then:
      - kind: SET_DECISION
        decision: REVIEW
      - kind: ADD_SCORE
        score_delta: 30
      - kind: EMIT
        severity: INFO
        code: HIGH_PLAFON
        field: fasilitas.plafon
        msg: "Plafon > 5M — VP approval required"
  - id: "risk-r002"
    priority: 20
    when: "fasilitas.kolektibilitas IN {'4', '5'}"
    then:
      - kind: ADD_SCORE
        score_delta: 50
      - kind: SET_DECISION
        decision: REJECT
      - kind: EMIT
        severity: ERROR
        code: BAD_COLLECTIBILITY
        field: fasilitas.kolektibilitas
        msg: "Kolektibilitas 4/5 — auto reject"
"#;

    #[test]
    fn test_vdicl_risk_scoring() {
        let input = json!({
            "fasilitas": {"plafon": 10000000000i64, "kolektibilitas": "5"}
        });
        let result = evaluate_rules(VDICL_RISK, &input).unwrap();
        assert_eq!(result.score, 80); // 30 + 50
        assert_eq!(result.decision, Some("REJECT".into())); // last SET_DECISION wins
        assert_eq!(result.findings.len(), 2);
        assert_eq!(result.rules_matched, 2);
    }

    #[test]
    fn test_vdicl_first_hit_policy() {
        let yaml = r#"
id: first_match_v1
hit_policy: FIRST
rules:
  - id: r1
    priority: 10
    when: "score > 80"
    then:
      - kind: SET_DECISION
        decision: APPROVE
  - id: r2
    priority: 20
    when: "score > 60"
    then:
      - kind: SET_DECISION
        decision: REVIEW
"#;
        let input = json!({"score": 85});
        let result = evaluate_rules(yaml, &input).unwrap();
        assert_eq!(result.decision, Some("APPROVE".into()));
        assert_eq!(result.rules_matched, 1); // FIRST: stops after r1
    }

    #[test]
    fn test_vdicl_disabled_rule() {
        let yaml = r#"
id: disabled_test_v1
rules:
  - id: r1
    enabled: false
    when: "true"
    then:
      - kind: EMIT
        severity: ERROR
        code: SHOULD_NOT_FIRE
        msg: "This should not fire"
  - id: r2
    enabled: true
    when: "true"
    then:
      - kind: EMIT
        severity: INFO
        code: SHOULD_FIRE
        msg: "This should fire"
"#;
        let input = json!({});
        let result = evaluate_rules(yaml, &input).unwrap();
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].code, "SHOULD_FIRE");
    }

    #[test]
    fn test_vdicl_set_output() {
        let yaml = r#"
id: set_test_v1
rules:
  - id: r1
    when: "amount > 1000"
    then:
      - kind: SET
        path: out.requires_approval
        value: true
      - kind: SET
        path: out.approval_level
        value: "VP"
"#;
        let input = json!({"amount": 5000});
        let result = evaluate_rules(yaml, &input).unwrap();
        assert_eq!(result.outputs["out.requires_approval"], json!(true));
        assert_eq!(result.outputs["out.approval_level"], json!("VP"));
    }

    #[test]
    fn test_vdicl_cross_field_slik() {
        let yaml = r#"
metadata:
  rulepack_id: "cross_field_v1"
rules:
  - id: r1
    when: "nasabah.status_kawin == 'K' AND (nasabah.nik_pasangan IS NULL OR ISBLANK(nasabah.nik_pasangan))"
    then:
      - kind: EMIT
        severity: ERROR
        code: PASANGAN_NIK_MISSING
        field: nasabah.nik_pasangan
        msg: "Status kawin K tapi NIK pasangan kosong"
  - id: r2
    when: "fasilitas.baki_debet > fasilitas.plafon"
    then:
      - kind: EMIT
        severity: ERROR
        code: BAKI_GT_PLAFON
        field: fasilitas.baki_debet
        msg: "Baki debet melebihi plafon"
"#;
        let input = json!({
            "nasabah": {"status_kawin": "K", "nik_pasangan": null, "nik": "1234"},
            "fasilitas": {"plafon": 1000000, "baki_debet": 2000000}
        });
        let result = evaluate_rules(yaml, &input).unwrap();
        assert_eq!(result.findings.len(), 2);
        assert_eq!(result.findings[0].code, "PASANGAN_NIK_MISSING");
        assert_eq!(result.findings[1].code, "BAKI_GT_PLAFON");
    }
}
