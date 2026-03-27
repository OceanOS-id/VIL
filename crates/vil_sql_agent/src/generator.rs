use crate::schema::SchemaRegistry;
use crate::validator::{validate_sql, SafeQuery};

/// Template-based SQL generator from natural language patterns.
pub struct SqlGenerator<'a> {
    registry: &'a SchemaRegistry,
}

impl<'a> SqlGenerator<'a> {
    pub fn new(registry: &'a SchemaRegistry) -> Self {
        Self { registry }
    }

    /// Generate a SELECT query for all rows in a table.
    pub fn select_all(&self, table_name: &str) -> Option<SafeQuery> {
        let table = self.registry.get(table_name)?;
        let sql = format!("SELECT {} FROM {}", table.column_names(), table.name);
        let validation = validate_sql(&sql);
        Some(SafeQuery {
            sql,
            params: Vec::new(),
            safe: validation.is_safe(),
        })
    }

    /// Generate a SELECT query with a WHERE clause on a single column.
    pub fn select_where(&self, table_name: &str, column: &str, placeholder: &str) -> Option<SafeQuery> {
        let table = self.registry.get(table_name)?;
        if !table.columns.iter().any(|c| c.name == column) {
            return None;
        }
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = {}",
            table.column_names(),
            table.name,
            column,
            placeholder,
        );
        let validation = validate_sql(&sql);
        Some(SafeQuery {
            sql,
            params: vec![placeholder.to_string()],
            safe: validation.is_safe(),
        })
    }

    /// Generate a COUNT query for a table.
    pub fn count(&self, table_name: &str) -> Option<SafeQuery> {
        let table = self.registry.get(table_name)?;
        let sql = format!("SELECT COUNT(*) FROM {}", table.name);
        let validation = validate_sql(&sql);
        Some(SafeQuery {
            sql,
            params: Vec::new(),
            safe: validation.is_safe(),
        })
    }

    /// Generate a SELECT with ORDER BY and LIMIT.
    pub fn select_ordered(
        &self,
        table_name: &str,
        order_col: &str,
        desc: bool,
        limit: usize,
    ) -> Option<SafeQuery> {
        let table = self.registry.get(table_name)?;
        if !table.columns.iter().any(|c| c.name == order_col) {
            return None;
        }
        let direction = if desc { "DESC" } else { "ASC" };
        let sql = format!(
            "SELECT {} FROM {} ORDER BY {} {} LIMIT {}",
            table.column_names(),
            table.name,
            order_col,
            direction,
            limit,
        );
        let validation = validate_sql(&sql);
        Some(SafeQuery {
            sql,
            params: Vec::new(),
            safe: validation.is_safe(),
        })
    }

    /// Attempt to generate SQL from a simple natural language query.
    /// Supports patterns like "show all <table>", "count <table>", "find <table> where <col> = <val>".
    pub fn from_natural_language(&self, query: &str) -> Option<SafeQuery> {
        let q = query.to_lowercase();

        // "count <table>"
        if q.starts_with("count ") {
            let table_name = q.strip_prefix("count ")?.trim();
            return self.count(table_name);
        }

        // "show all <table>" or "list all <table>"
        if q.starts_with("show all ") || q.starts_with("list all ") {
            let table_name = q.split_whitespace().nth(2)?;
            return self.select_all(table_name);
        }

        // "find <table> where <col> = <val>"
        if q.starts_with("find ") {
            let parts: Vec<&str> = q.splitn(4, ' ').collect();
            if parts.len() >= 4 && parts[2] == "where" {
                let table_name = parts[1];
                let rest = parts[3];
                if let Some((col, _val)) = rest.split_once(" = ") {
                    return self.select_where(table_name, col.trim(), "$1");
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::TableSchema;

    fn make_registry() -> SchemaRegistry {
        let mut reg = SchemaRegistry::new();
        reg.register(
            TableSchema::new("users")
                .column("id", "INTEGER", false, true)
                .column("name", "VARCHAR(255)", false, false)
                .column("email", "VARCHAR(255)", true, false),
        );
        reg
    }

    #[test]
    fn test_select_all() {
        let reg = make_registry();
        let gen = SqlGenerator::new(&reg);
        let sq = gen.select_all("users").unwrap();
        assert!(sq.sql.contains("SELECT"));
        assert!(sq.sql.contains("FROM users"));
        assert!(sq.safe);
    }

    #[test]
    fn test_select_where() {
        let reg = make_registry();
        let gen = SqlGenerator::new(&reg);
        let sq = gen.select_where("users", "id", "$1").unwrap();
        assert!(sq.sql.contains("WHERE id = $1"));
        assert!(sq.safe);
    }

    #[test]
    fn test_select_where_invalid_column() {
        let reg = make_registry();
        let gen = SqlGenerator::new(&reg);
        assert!(gen.select_where("users", "nonexistent", "$1").is_none());
    }

    #[test]
    fn test_count() {
        let reg = make_registry();
        let gen = SqlGenerator::new(&reg);
        let sq = gen.count("users").unwrap();
        assert!(sq.sql.contains("COUNT(*)"));
        assert!(sq.safe);
    }

    #[test]
    fn test_select_ordered() {
        let reg = make_registry();
        let gen = SqlGenerator::new(&reg);
        let sq = gen.select_ordered("users", "name", true, 10).unwrap();
        assert!(sq.sql.contains("ORDER BY name DESC"));
        assert!(sq.sql.contains("LIMIT 10"));
    }

    #[test]
    fn test_natural_language_count() {
        let reg = make_registry();
        let gen = SqlGenerator::new(&reg);
        let sq = gen.from_natural_language("count users").unwrap();
        assert!(sq.sql.contains("COUNT(*)"));
    }

    #[test]
    fn test_natural_language_show_all() {
        let reg = make_registry();
        let gen = SqlGenerator::new(&reg);
        let sq = gen.from_natural_language("show all users").unwrap();
        assert!(sq.sql.contains("SELECT"));
        assert!(sq.sql.contains("FROM users"));
    }

    #[test]
    fn test_nonexistent_table() {
        let reg = make_registry();
        let gen = SqlGenerator::new(&reg);
        assert!(gen.select_all("products").is_none());
    }
}
