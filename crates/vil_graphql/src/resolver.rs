// =============================================================================
// VIL GraphQL — CRUD Resolvers
// =============================================================================
//
// Auto-generated resolvers that delegate to DbProvider.
// Each resolver = 1 vtable call to the database provider.

use std::sync::Arc;
use vil_db_semantic::{DbProvider, ToSqlValue, DbResult};

/// Generic CRUD resolver that works with any entity.
pub struct CrudResolver {
    provider: Arc<dyn DbProvider>,
    table: String,
    primary_key: String,
    fields: Vec<String>,
}

impl CrudResolver {
    pub fn new(
        provider: Arc<dyn DbProvider>,
        table: &str,
        primary_key: &str,
        fields: Vec<String>,
    ) -> Self {
        Self {
            provider,
            table: table.to_string(),
            primary_key: primary_key.to_string(),
            fields,
        }
    }

    /// Query: find by ID.
    pub async fn find_by_id(&self, id: i64) -> DbResult<Option<serde_json::Value>> {
        let data = self.provider.find_one(
            &self.table, &self.primary_key, &ToSqlValue::Int(id)
        ).await?;

        match data {
            Some(bytes) => {
                let value = serde_json::from_slice(&bytes)
                    .unwrap_or(serde_json::Value::Null);
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Query: find many with optional filter.
    pub async fn find_many(
        &self, filter: &str, params: &[ToSqlValue], limit: usize, offset: usize,
    ) -> DbResult<Vec<serde_json::Value>> {
        let full_filter = if filter.is_empty() {
            format!("1=1 LIMIT {} OFFSET {}", limit, offset)
        } else {
            format!("{} LIMIT {} OFFSET {}", filter, limit, offset)
        };

        let rows = self.provider.find_many(&self.table, &full_filter, params).await?;

        Ok(rows.iter().filter_map(|bytes| {
            serde_json::from_slice(bytes).ok()
        }).collect())
    }

    /// Query: count.
    pub async fn count(&self, filter: &str) -> DbResult<u64> {
        self.provider.count(&self.table, filter, &[]).await
    }

    /// Mutation: insert.
    pub async fn create(&self, input: &serde_json::Value) -> DbResult<i64> {
        let (fields, values) = json_to_fields_values(input, &self.fields);
        let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
        self.provider.insert(&self.table, &field_refs, &values).await
    }

    /// Mutation: update by ID.
    pub async fn update(&self, id: i64, input: &serde_json::Value) -> DbResult<u64> {
        let (fields, values) = json_to_fields_values(input, &self.fields);
        let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
        self.provider.update(
            &self.table, &self.primary_key, &ToSqlValue::Int(id),
            &field_refs, &values,
        ).await
    }

    /// Mutation: delete by ID.
    pub async fn delete(&self, id: i64) -> DbResult<bool> {
        self.provider.delete(&self.table, &self.primary_key, &ToSqlValue::Int(id)).await
    }
}

/// Convert JSON object to field names + ToSqlValue arrays.
fn json_to_fields_values(
    json: &serde_json::Value,
    known_fields: &[String],
) -> (Vec<String>, Vec<ToSqlValue>) {
    let mut fields = Vec::new();
    let mut values = Vec::new();

    if let Some(obj) = json.as_object() {
        for field in known_fields {
            if let Some(val) = obj.get(field) {
                fields.push(field.clone());
                values.push(json_to_sql_value(val));
            }
        }
    }

    (fields, values)
}

fn json_to_sql_value(val: &serde_json::Value) -> ToSqlValue {
    match val {
        serde_json::Value::Null => ToSqlValue::Null,
        serde_json::Value::Bool(b) => ToSqlValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { ToSqlValue::Int(i) }
            else if let Some(f) = n.as_f64() { ToSqlValue::Float(f) }
            else { ToSqlValue::Null }
        }
        serde_json::Value::String(s) => ToSqlValue::Text(s.clone()),
        _ => ToSqlValue::Text(val.to_string()),
    }
}
