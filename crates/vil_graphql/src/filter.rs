// =============================================================================
// VIL GraphQL — Filter Types
// =============================================================================

use serde::{Deserialize, Serialize};
use vil_db_semantic::ToSqlValue;

/// Generic filter for integer fields.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntFilter {
    pub eq: Option<i64>,
    pub gt: Option<i64>,
    pub lt: Option<i64>,
    pub gte: Option<i64>,
    pub lte: Option<i64>,
    #[serde(rename = "in")]
    pub in_values: Option<Vec<i64>>,
}

/// Generic filter for string fields.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StringFilter {
    pub eq: Option<String>,
    pub contains: Option<String>,
    pub starts_with: Option<String>,
    pub ends_with: Option<String>,
    #[serde(rename = "in")]
    pub in_values: Option<Vec<String>>,
}

/// Generic filter for float fields.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FloatFilter {
    pub eq: Option<f64>,
    pub gt: Option<f64>,
    pub lt: Option<f64>,
}

/// Generic filter for boolean fields.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BoolFilter {
    pub eq: Option<bool>,
}

/// Build SQL WHERE clause from filters.
pub fn build_where_clause(filters: &serde_json::Value) -> (String, Vec<ToSqlValue>) {
    let mut clauses = Vec::new();
    let mut params = Vec::new();

    if let Some(obj) = filters.as_object() {
        for (field, filter) in obj {
            if let Some(filter_obj) = filter.as_object() {
                if let Some(eq) = filter_obj.get("eq") {
                    clauses.push(format!("{} = ?", field));
                    params.push(json_val_to_sql(eq));
                }
                if let Some(gt) = filter_obj.get("gt") {
                    clauses.push(format!("{} > ?", field));
                    params.push(json_val_to_sql(gt));
                }
                if let Some(lt) = filter_obj.get("lt") {
                    clauses.push(format!("{} < ?", field));
                    params.push(json_val_to_sql(lt));
                }
                if let Some(contains) = filter_obj.get("contains") {
                    if let Some(s) = contains.as_str() {
                        clauses.push(format!("{} LIKE ?", field));
                        params.push(ToSqlValue::Text(format!("%{}%", s)));
                    }
                }
            }
        }
    }

    let where_str = if clauses.is_empty() {
        "1=1".to_string()
    } else {
        clauses.join(" AND ")
    };

    (where_str, params)
}

fn json_val_to_sql(val: &serde_json::Value) -> ToSqlValue {
    match val {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ToSqlValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                ToSqlValue::Float(f)
            } else {
                ToSqlValue::Null
            }
        }
        serde_json::Value::String(s) => ToSqlValue::Text(s.clone()),
        serde_json::Value::Bool(b) => ToSqlValue::Bool(*b),
        _ => ToSqlValue::Null,
    }
}
