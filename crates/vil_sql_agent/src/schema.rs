use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A column definition in a table schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    /// Column name.
    pub name: String,
    /// SQL data type (e.g., "INTEGER", "VARCHAR(255)", "TIMESTAMP").
    pub data_type: String,
    /// Whether this column allows NULL values.
    pub nullable: bool,
    /// Whether this column is a primary key.
    pub primary_key: bool,
}

/// A table schema with columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    /// Table name.
    pub name: String,
    /// Columns in this table.
    pub columns: Vec<Column>,
}

impl TableSchema {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            columns: Vec::new(),
        }
    }

    /// Add a column to this table schema.
    pub fn column(
        mut self,
        name: &str,
        data_type: &str,
        nullable: bool,
        primary_key: bool,
    ) -> Self {
        self.columns.push(Column {
            name: name.to_string(),
            data_type: data_type.to_string(),
            nullable,
            primary_key,
        });
        self
    }

    /// Get the primary key column(s).
    pub fn primary_keys(&self) -> Vec<&Column> {
        self.columns.iter().filter(|c| c.primary_key).collect()
    }

    /// Get column names as a comma-separated string.
    pub fn column_names(&self) -> String {
        self.columns
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Registry of table schemas.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaRegistry {
    pub tables: HashMap<String, TableSchema>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a table schema.
    pub fn register(&mut self, table: TableSchema) {
        self.tables.insert(table.name.clone(), table);
    }

    /// Look up a table by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&TableSchema> {
        let lower = name.to_lowercase();
        self.tables
            .values()
            .find(|t| t.name.to_lowercase() == lower)
    }

    /// List all registered table names.
    pub fn table_names(&self) -> Vec<&str> {
        self.tables.keys().map(|s| s.as_str()).collect()
    }

    /// Generate a text summary of all schemas for LLM context.
    pub fn to_schema_text(&self) -> String {
        let mut out = String::new();
        for table in self.tables.values() {
            out.push_str(&format!("TABLE {}\n", table.name));
            for col in &table.columns {
                let pk = if col.primary_key { " PRIMARY KEY" } else { "" };
                let null = if col.nullable { " NULL" } else { " NOT NULL" };
                out.push_str(&format!("  {} {}{}{}\n", col.name, col.data_type, null, pk));
            }
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_registry() -> SchemaRegistry {
        let mut reg = SchemaRegistry::new();
        reg.register(
            TableSchema::new("users")
                .column("id", "INTEGER", false, true)
                .column("name", "VARCHAR(255)", false, false)
                .column("email", "VARCHAR(255)", true, false),
        );
        reg.register(
            TableSchema::new("orders")
                .column("id", "INTEGER", false, true)
                .column("user_id", "INTEGER", false, false)
                .column("total", "DECIMAL(10,2)", false, false)
                .column("created_at", "TIMESTAMP", false, false),
        );
        reg
    }

    #[test]
    fn test_schema_building() {
        let table = TableSchema::new("users")
            .column("id", "INTEGER", false, true)
            .column("name", "VARCHAR(255)", false, false);
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.primary_keys().len(), 1);
    }

    #[test]
    fn test_column_names() {
        let table = TableSchema::new("t")
            .column("a", "INT", false, false)
            .column("b", "TEXT", false, false);
        assert_eq!(table.column_names(), "a, b");
    }

    #[test]
    fn test_registry_lookup() {
        let reg = make_registry();
        assert!(reg.get("users").is_some());
        assert!(reg.get("Users").is_some()); // case-insensitive
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_table_names() {
        let reg = make_registry();
        let names = reg.table_names();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_schema_text() {
        let reg = make_registry();
        let text = reg.to_schema_text();
        assert!(text.contains("TABLE users"));
        assert!(text.contains("TABLE orders"));
        assert!(text.contains("PRIMARY KEY"));
    }
}
