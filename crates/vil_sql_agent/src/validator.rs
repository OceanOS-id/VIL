use regex::Regex;
use serde::{Deserialize, Serialize};

/// Severity of a SQL validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// No issues detected.
    Safe,
    /// Potentially risky but not necessarily malicious.
    Warning,
    /// Likely SQL injection or destructive operation.
    Dangerous,
}

/// Result of SQL validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Overall severity.
    pub severity: Severity,
    /// List of issues found.
    pub issues: Vec<String>,
    /// The original SQL that was validated.
    pub sql: String,
}

impl ValidationResult {
    pub fn is_safe(&self) -> bool {
        self.severity == Severity::Safe
    }
}

/// A validated, safe query with parameterized placeholders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeQuery {
    /// The SQL statement (with $1, $2, ... placeholders).
    pub sql: String,
    /// Parameter values.
    pub params: Vec<String>,
    /// Whether the query passed safety validation.
    pub safe: bool,
}

/// SQL injection patterns to detect.
struct InjectionPatterns {
    drop_table: Regex,
    delete_from: Regex,
    truncate: Regex,
    union_select: Regex,
    comment_dash: Regex,
    comment_hash: Regex,
    semicolon_chain: Regex,
    alter_table: Regex,
    exec_xp: Regex,
    insert_into: Regex,
    update_set: Regex,
}

impl InjectionPatterns {
    fn new() -> Self {
        Self {
            drop_table: Regex::new(r"(?i)\bDROP\s+(TABLE|DATABASE|INDEX|VIEW)\b").unwrap(),
            delete_from: Regex::new(r"(?i)\bDELETE\s+FROM\b").unwrap(),
            truncate: Regex::new(r"(?i)\bTRUNCATE\s+TABLE\b").unwrap(),
            union_select: Regex::new(r"(?i)\bUNION\s+(ALL\s+)?SELECT\b").unwrap(),
            comment_dash: Regex::new(r"--").unwrap(),
            comment_hash: Regex::new(r"#[^\n]*$").unwrap(),
            semicolon_chain: Regex::new(r";\s*\S").unwrap(),
            alter_table: Regex::new(r"(?i)\bALTER\s+TABLE\b").unwrap(),
            exec_xp: Regex::new(r"(?i)\b(EXEC|EXECUTE|xp_)\b").unwrap(),
            insert_into: Regex::new(r"(?i)\bINSERT\s+INTO\b").unwrap(),
            update_set: Regex::new(r"(?i)\bUPDATE\s+\w+\s+SET\b").unwrap(),
        }
    }
}

/// Validate a SQL statement for potential injection or destructive operations.
pub fn validate_sql(sql: &str) -> ValidationResult {
    let patterns = InjectionPatterns::new();
    let mut issues = Vec::new();
    let mut severity = Severity::Safe;

    // Dangerous patterns
    if patterns.drop_table.is_match(sql) {
        issues.push("DROP statement detected".into());
        severity = Severity::Dangerous;
    }
    if patterns.delete_from.is_match(sql) {
        issues.push("DELETE FROM statement detected".into());
        severity = Severity::Dangerous;
    }
    if patterns.truncate.is_match(sql) {
        issues.push("TRUNCATE TABLE detected".into());
        severity = Severity::Dangerous;
    }
    if patterns.union_select.is_match(sql) {
        issues.push("UNION SELECT detected (potential injection)".into());
        severity = Severity::Dangerous;
    }
    if patterns.exec_xp.is_match(sql) {
        issues.push("EXEC/xp_ command detected".into());
        severity = Severity::Dangerous;
    }

    // Warning patterns (only escalate if not already Dangerous)
    if patterns.comment_dash.is_match(sql) {
        issues.push("SQL comment (--) detected".into());
        if severity != Severity::Dangerous {
            severity = Severity::Warning;
        }
    }
    if patterns.comment_hash.is_match(sql) {
        issues.push("SQL comment (#) detected".into());
        if severity != Severity::Dangerous {
            severity = Severity::Warning;
        }
    }
    if patterns.semicolon_chain.is_match(sql) {
        issues.push("Multiple statements (;) detected".into());
        if severity != Severity::Dangerous {
            severity = Severity::Dangerous;
        }
    }
    if patterns.alter_table.is_match(sql) {
        issues.push("ALTER TABLE detected".into());
        if severity == Severity::Safe {
            severity = Severity::Warning;
        }
    }
    if patterns.insert_into.is_match(sql) {
        issues.push("INSERT INTO detected — ensure parameterized".into());
        if severity == Severity::Safe {
            severity = Severity::Warning;
        }
    }
    if patterns.update_set.is_match(sql) {
        issues.push("UPDATE SET detected — ensure parameterized".into());
        if severity == Severity::Safe {
            severity = Severity::Warning;
        }
    }

    ValidationResult {
        severity,
        issues,
        sql: sql.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_select() {
        let r = validate_sql("SELECT id, name FROM users WHERE id = $1");
        assert_eq!(r.severity, Severity::Safe);
        assert!(r.issues.is_empty());
    }

    #[test]
    fn test_drop_table() {
        let r = validate_sql("DROP TABLE users");
        assert_eq!(r.severity, Severity::Dangerous);
        assert!(r.issues.iter().any(|i| i.contains("DROP")));
    }

    #[test]
    fn test_drop_table_case_insensitive() {
        let r = validate_sql("drop table users");
        assert_eq!(r.severity, Severity::Dangerous);
    }

    #[test]
    fn test_union_select() {
        let r = validate_sql("SELECT * FROM users WHERE id = 1 UNION SELECT * FROM passwords");
        assert_eq!(r.severity, Severity::Dangerous);
        assert!(r.issues.iter().any(|i| i.contains("UNION")));
    }

    #[test]
    fn test_comment_dash() {
        let r = validate_sql("SELECT * FROM users WHERE id = 1 -- AND admin = 1");
        assert!(r.severity == Severity::Warning || r.severity == Severity::Dangerous);
        assert!(r.issues.iter().any(|i| i.contains("--")));
    }

    #[test]
    fn test_semicolon_chain() {
        let r = validate_sql("SELECT 1; DROP TABLE users");
        assert_eq!(r.severity, Severity::Dangerous);
    }

    #[test]
    fn test_delete_from() {
        let r = validate_sql("DELETE FROM users WHERE 1=1");
        assert_eq!(r.severity, Severity::Dangerous);
    }

    #[test]
    fn test_insert_warning() {
        let r = validate_sql("INSERT INTO users (name) VALUES ('test')");
        assert_eq!(r.severity, Severity::Warning);
    }

    #[test]
    fn test_update_warning() {
        let r = validate_sql("UPDATE users SET name = 'test' WHERE id = 1");
        assert_eq!(r.severity, Severity::Warning);
    }

    #[test]
    fn test_safe_query_struct() {
        let sq = SafeQuery {
            sql: "SELECT * FROM users WHERE id = $1".into(),
            params: vec!["42".into()],
            safe: true,
        };
        assert!(sq.safe);
        assert_eq!(sq.params.len(), 1);
    }
}
