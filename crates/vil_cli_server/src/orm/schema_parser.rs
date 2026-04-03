//! SQL DDL Schema Parser — Parses CREATE TABLE statements into structured metadata.
//!
//! Supports SQLite and PostgreSQL DDL syntax:
//! - Column types: TEXT, INTEGER, REAL, BOOLEAN, BLOB
//! - Constraints: PRIMARY KEY, NOT NULL, UNIQUE, DEFAULT, REFERENCES, CHECK
//! - Table constraints: UNIQUE(col1, col2), PRIMARY KEY(col), FOREIGN KEY
//! - DEFAULT expressions: literals, strftime(), function calls

/// Metadata for a single database table.
#[derive(Debug, Clone)]
pub struct TableMeta {
    pub name: String,
    pub columns: Vec<ColumnMeta>,
    pub primary_keys: Vec<String>,
    pub unique_constraints: Vec<Vec<String>>,
    pub foreign_keys: Vec<ForeignKeyMeta>,
}

impl TableMeta {
    /// True if this table has a composite primary key (2+ columns).
    pub fn is_composite_pk(&self) -> bool {
        self.primary_keys.len() > 1
    }

    /// First PK column name (for backward compat with single-PK code).
    pub fn first_pk(&self) -> &str {
        self.primary_keys.first().map(|s| s.as_str()).unwrap_or("id")
    }
}

/// Metadata for a single column.
#[derive(Debug, Clone)]
pub struct ColumnMeta {
    pub name: String,
    pub sql_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
    pub is_unique: bool,
    pub is_not_null: bool,
    pub check_expr: Option<String>,
    pub references: Option<ForeignKeyMeta>,
}

/// Foreign key reference.
#[derive(Debug, Clone)]
pub struct ForeignKeyMeta {
    pub table: String,
    pub column: String,
    pub on_delete: Option<String>,
}

impl TableMeta {
    /// Get columns excluding auto-generated ones (for INSERT).
    pub fn insertable_columns(&self) -> Vec<&ColumnMeta> {
        self.columns.iter().filter(|c| {
            !c.is_auto_timestamp() && !c.is_primary_key
        }).collect()
    }

    /// Get columns suitable for list view (exclude large TEXT that might be content/body).
    pub fn list_columns(&self) -> Vec<&ColumnMeta> {
        self.columns.iter().filter(|c| {
            !c.is_large_text_hint()
        }).collect()
    }
}

impl ColumnMeta {
    /// Detect if this is an auto-timestamp column (created_at, updated_at with strftime default).
    pub fn is_auto_timestamp(&self) -> bool {
        if let Some(ref def) = self.default_value {
            def.contains("strftime") && (self.name == "created_at" || self.name == "updated_at")
        } else {
            false
        }
    }

    /// Detect if this is a created_at auto-timestamp.
    pub fn is_created_at(&self) -> bool {
        self.name == "created_at" && self.default_value.as_ref().map(|d| d.contains("strftime")).unwrap_or(false)
    }

    /// Detect if this is an updated_at auto-timestamp.
    pub fn is_updated_at(&self) -> bool {
        self.name == "updated_at" && self.default_value.as_ref().map(|d| d.contains("strftime")).unwrap_or(false)
    }

    /// Detect if column name suggests sensitive data (password, hash, secret, token).
    pub fn is_sensitive(&self) -> bool {
        let lower = self.name.to_lowercase();
        lower.contains("password") || lower.contains("hash") || lower.contains("secret")
            || (lower.contains("token") && lower != "fcm_token")
    }

    /// Detect if this is likely a large text field (content, body, essay, passage).
    pub fn is_large_text_hint(&self) -> bool {
        if self.sql_type.to_uppercase() != "TEXT" { return false; }
        let lower = self.name.to_lowercase();
        lower.contains("content") || lower.contains("body") || lower.contains("essay")
            || lower.contains("passage") || lower.contains("stack_trace")
            || lower.contains("annotations") || lower.contains("highlights")
    }

    /// Map SQL type to Rust type.
    pub fn rust_type(&self) -> &'static str {
        match self.sql_type.to_uppercase().as_str() {
            "TEXT" => "String",
            "INTEGER" | "INT" | "BIGINT" | "SMALLINT" => "i64",
            "REAL" | "FLOAT" | "DOUBLE" | "NUMERIC" | "DECIMAL" => "f64",
            "BOOLEAN" | "BOOL" => "i64",
            "BLOB" | "BYTEA" => "Vec<u8>",
            _ => "String",
        }
    }

    /// Full Rust type considering nullability.
    /// Fields with DEFAULT values are non-nullable (DB fills the default).
    /// Only fields without NOT NULL AND without DEFAULT are Option<T>.
    pub fn rust_type_full(&self) -> String {
        let base = self.rust_type();
        let truly_nullable = self.nullable && !self.is_primary_key && self.default_value.is_none();
        if truly_nullable {
            format!("Option<{}>", base)
        } else {
            base.to_string()
        }
    }
}

// =============================================================================
// Parser
// =============================================================================

/// Parse a SQL file containing CREATE TABLE statements.
/// Returns one TableMeta per table found.
pub fn parse_schema(sql: &str) -> Vec<TableMeta> {
    let mut tables = Vec::new();

    // Extract each CREATE TABLE ... ); block
    let normalized = sql.replace('\r', "");
    let mut remaining = normalized.as_str();

    while let Some(start) = remaining.to_uppercase().find("CREATE TABLE") {
        remaining = &remaining[start..];

        // Find matching closing );
        let body_start = match remaining.find('(') {
            Some(p) => p,
            None => break,
        };

        // Track parenthesis depth to find the closing )
        let mut depth = 0;
        let mut body_end = body_start;
        for (i, ch) in remaining[body_start..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        body_end = body_start + i;
                        break;
                    }
                }
                _ => {}
            }
        }

        if body_end <= body_start {
            remaining = &remaining[body_start + 1..];
            continue;
        }

        // Extract table name
        let header = &remaining[..body_start].trim();
        let table_name = extract_table_name(header);

        // Extract column definitions
        let body = &remaining[body_start + 1..body_end];
        let table = parse_table_body(&table_name, body);
        tables.push(table);

        remaining = &remaining[body_end + 1..];
    }

    tables
}

/// Extract table name from "CREATE TABLE [IF NOT EXISTS] name"
fn extract_table_name(header: &str) -> String {
    let upper = header.to_uppercase();
    let after_table = if let Some(pos) = upper.find("TABLE") {
        &header[pos + 5..]
    } else {
        header
    };

    let cleaned = after_table.trim();
    let without_if = if cleaned.to_uppercase().starts_with("IF NOT EXISTS") {
        cleaned[13..].trim()
    } else {
        cleaned
    };

    without_if.trim().to_string()
}

/// Parse the body inside CREATE TABLE (...) into columns and constraints.
fn parse_table_body(table_name: &str, body: &str) -> TableMeta {
    let mut columns = Vec::new();
    let mut primary_keys: Vec<String> = Vec::new();
    let mut unique_constraints: Vec<Vec<String>> = Vec::new();
    let mut foreign_keys: Vec<ForeignKeyMeta> = Vec::new();

    let parts = split_by_comma_respecting_parens(body);

    for part in &parts {
        let trimmed = part.trim();
        if trimmed.is_empty() { continue; }

        let upper = trimmed.to_uppercase();

        // Table-level PRIMARY KEY constraint: PRIMARY KEY (col1, col2)
        if upper.starts_with("PRIMARY KEY") {
            if let Some(cols) = extract_parens_content(trimmed) {
                let pk_cols: Vec<String> = cols.split(',').map(|s| s.trim().to_string()).collect();
                primary_keys = pk_cols;
            }
            continue;
        }

        if upper.starts_with("UNIQUE") {
            if let Some(cols) = extract_parens_content(trimmed) {
                let unique_cols: Vec<String> = cols.split(',').map(|s| s.trim().to_string()).collect();
                unique_constraints.push(unique_cols);
            }
            continue;
        }

        if upper.starts_with("FOREIGN KEY") {
            if let Some(fk) = parse_foreign_key_constraint(trimmed) {
                foreign_keys.push(fk);
            }
            continue;
        }

        // Column definition
        if let Some(col) = parse_column_def(trimmed) {
            if col.is_primary_key && primary_keys.is_empty() {
                primary_keys.push(col.name.clone());
            }
            columns.push(col);
        }
    }

    // Apply table-level unique constraints to columns
    for uc in &unique_constraints {
        if uc.len() == 1 {
            if let Some(col) = columns.iter_mut().find(|c| c.name == uc[0]) {
                col.is_unique = true;
            }
        }
    }

    // Mark PK columns
    for pk_name in &primary_keys {
        if let Some(col) = columns.iter_mut().find(|c| &c.name == pk_name) {
            col.is_primary_key = true;
        }
    }

    // Default to first column if no PK found
    if primary_keys.is_empty() {
        if let Some(col) = columns.first() {
            primary_keys.push(col.name.clone());
        }
    }

    TableMeta {
        name: table_name.to_string(),
        columns,
        primary_keys,
        unique_constraints,
        foreign_keys,
    }
}

/// Parse a single column definition line.
fn parse_column_def(line: &str) -> Option<ColumnMeta> {
    let tokens = tokenize_column(line);
    if tokens.is_empty() { return None; }

    let name = tokens[0].clone();
    // Skip if it looks like a constraint keyword
    let upper_name = name.to_uppercase();
    if upper_name == "CONSTRAINT" || upper_name == "CHECK" || upper_name == "FOREIGN" {
        return None;
    }

    let sql_type = if tokens.len() > 1 {
        tokens[1].to_uppercase()
    } else {
        "TEXT".to_string()
    };

    let upper_line = line.to_uppercase();
    let is_primary_key = upper_line.contains("PRIMARY KEY");
    let is_not_null = upper_line.contains("NOT NULL") || is_primary_key;
    let is_unique = upper_line.contains(" UNIQUE") && !upper_line.starts_with("UNIQUE");
    let nullable = !is_not_null && !is_primary_key;

    // Extract DEFAULT value
    let default_value = extract_default_value(line);

    // Extract CHECK expression
    let check_expr = extract_check_expr(line);

    // Extract REFERENCES
    let references = extract_inline_reference(line);

    Some(ColumnMeta {
        name,
        sql_type,
        nullable,
        default_value,
        is_primary_key,
        is_unique,
        is_not_null,
        check_expr,
        references,
    })
}

/// Tokenize a column definition, respecting parentheses.
fn tokenize_column(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_quote = false;

    for ch in line.chars() {
        match ch {
            '\'' if depth == 0 => {
                in_quote = !in_quote;
                current.push(ch);
            }
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ' ' | '\t' if depth == 0 && !in_quote => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Split by comma, respecting parentheses depth.
fn split_by_comma_respecting_parens(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_quote = false;

    for ch in s.chars() {
        match ch {
            '\'' => {
                in_quote = !in_quote;
                current.push(ch);
            }
            '(' if !in_quote => {
                depth += 1;
                current.push(ch);
            }
            ')' if !in_quote => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 && !in_quote => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            '\n' | '\r' => {
                current.push(' ');
            }
            _ => current.push(ch),
        }
    }
    let last = current.trim().to_string();
    if !last.is_empty() {
        parts.push(last);
    }
    parts
}

/// Extract content inside first pair of parentheses.
fn extract_parens_content(s: &str) -> Option<String> {
    let start = s.find('(')?;
    let mut depth = 0;
    for (i, ch) in s[start..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[start + 1..start + i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Extract DEFAULT value from column definition.
fn extract_default_value(line: &str) -> Option<String> {
    let upper = line.to_uppercase();
    let pos = upper.find("DEFAULT ")?;
    let after = &line[pos + 8..];

    // Value could be: literal, 'string', number, (expression)
    let trimmed = after.trim();

    if trimmed.starts_with('(') {
        // Expression in parens — find matching close
        let content = extract_parens_content(trimmed)?;
        Some(format!("({})", content))
    } else if trimmed.starts_with('\'') {
        // Quoted string
        let end = trimmed[1..].find('\'')?;
        Some(trimmed[..end + 2].to_string())
    } else {
        // Number or keyword — take until space, comma, or CHECK
        let end = trimmed.find(|c: char| c == ' ' || c == ',' || c == ')').unwrap_or(trimmed.len());
        let val = trimmed[..end].trim().to_string();
        if val.is_empty() { None } else { Some(val) }
    }
}

/// Extract CHECK expression from column definition.
fn extract_check_expr(line: &str) -> Option<String> {
    let upper = line.to_uppercase();
    let pos = upper.find("CHECK ")?;
    let after = &line[pos + 6..];
    extract_parens_content(after.trim())
}

/// Extract inline REFERENCES from column definition.
fn extract_inline_reference(line: &str) -> Option<ForeignKeyMeta> {
    let upper = line.to_uppercase();
    let pos = upper.find("REFERENCES ")?;
    let after = &line[pos + 11..].trim();

    // Format: table(column) [ON DELETE action]
    let paren_start = after.find('(')?;
    let table = after[..paren_start].trim().to_string();
    let col_content = extract_parens_content(after)?;
    let column = col_content.trim().to_string();

    let on_delete = if let Some(od_pos) = upper[pos..].find("ON DELETE") {
        let after_od = line[pos + od_pos + 9..].trim();
        let end = after_od.find(|c: char| c == ',' || c == ')' || c == '\n').unwrap_or(after_od.len());
        Some(after_od[..end].trim().to_string())
    } else {
        None
    };

    Some(ForeignKeyMeta { table, column, on_delete })
}

/// Parse a table-level FOREIGN KEY constraint.
fn parse_foreign_key_constraint(line: &str) -> Option<ForeignKeyMeta> {
    // FOREIGN KEY (col) REFERENCES table(col) [ON DELETE action]
    extract_inline_reference(line)
}

// =============================================================================
// Display
// =============================================================================

impl std::fmt::Display for TableMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "TABLE {} (pk={}, {} cols, {} unique, {} fk)",
            self.name, self.primary_keys.join(","), self.columns.len(),
            self.unique_constraints.len(), self.foreign_keys.len())?;
        for col in &self.columns {
            let mut flags: Vec<String> = Vec::new();
            if col.is_primary_key { flags.push("PK".into()); }
            if col.is_unique { flags.push("UNIQUE".into()); }
            if col.is_not_null { flags.push("NOT NULL".into()); }
            if col.nullable { flags.push("NULL".into()); }
            if col.is_sensitive() { flags.push("SENSITIVE".into()); }
            if col.is_auto_timestamp() { flags.push("AUTO_TS".into()); }
            if let Some(ref def) = col.default_value { flags.push(format!("DEFAULT={}", def)); }
            if let Some(ref fk) = col.references {
                flags.push(format!("FK→{}.{}", fk.table, fk.column));
            }
            writeln!(f, "  {} {} → {} [{}]",
                col.name, col.sql_type, col.rust_type_full(),
                flags.join(", "))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS profiles (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE,
    full_name TEXT,
    avatar_url TEXT,
    bio TEXT,
    friend_code TEXT UNIQUE,
    hearts_count INTEGER DEFAULT 5,
    xp INTEGER DEFAULT 0,
    subscription_tier TEXT DEFAULT 'free',
    fcm_token TEXT,
    password_hash TEXT NOT NULL,
    peer_review_prefs TEXT,
    created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE IF NOT EXISTS quiz_results (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    date TEXT NOT NULL,
    skill_id TEXT,
    section TEXT NOT NULL,
    score INTEGER NOT NULL,
    correct_count INTEGER NOT NULL,
    total_questions INTEGER NOT NULL,
    xp_earned INTEGER NOT NULL,
    breakdown TEXT
);

CREATE TABLE IF NOT EXISTS ai_token_usage (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    date TEXT NOT NULL,
    tokens_used INTEGER DEFAULT 0,
    tokens_limit INTEGER NOT NULL,
    feature TEXT,
    UNIQUE(user_id, date)
);

CREATE TABLE IF NOT EXISTS question_bank (
    id TEXT PRIMARY KEY,
    skill_id INTEGER NOT NULL,
    section TEXT NOT NULL CHECK (section IN ('structure','written','reading','listening')),
    interaction TEXT NOT NULL CHECK (interaction IN ('fill_blank','identify_error','multiple_choice')),
    prompt TEXT NOT NULL,
    choices TEXT,
    cefr_target TEXT CHECK (cefr_target IN ('A2','B1','B2','C1')),
    difficulty_score INTEGER,
    passage_id TEXT REFERENCES passages(id),
    created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
    "#;

    #[test]
    fn test_parse_table_count() {
        let tables = parse_schema(SAMPLE_SQL);
        assert_eq!(tables.len(), 4);
    }

    #[test]
    fn test_profiles_table() {
        let tables = parse_schema(SAMPLE_SQL);
        let profiles = tables.iter().find(|t| t.name == "profiles").unwrap();
        
        assert_eq!(profiles.primary_keys[0], "id");
        assert_eq!(profiles.columns.len(), 14);
        
        let username = profiles.columns.iter().find(|c| c.name == "username").unwrap();
        assert!(username.is_unique);
        assert!(username.nullable); // TEXT without NOT NULL
        assert_eq!(username.rust_type_full(), "Option<String>");
        
        let pw = profiles.columns.iter().find(|c| c.name == "password_hash").unwrap();
        assert!(pw.is_sensitive());
        assert!(pw.is_not_null);
        assert_eq!(pw.rust_type_full(), "String");
        
        let created = profiles.columns.iter().find(|c| c.name == "created_at").unwrap();
        assert!(created.is_created_at());
        assert!(created.is_auto_timestamp());
        
        let updated = profiles.columns.iter().find(|c| c.name == "updated_at").unwrap();
        assert!(updated.is_updated_at());
        
        let xp = profiles.columns.iter().find(|c| c.name == "xp").unwrap();
        assert_eq!(xp.sql_type, "INTEGER");
        assert_eq!(xp.rust_type(), "i64");
        assert_eq!(xp.default_value, Some("0".to_string()));
    }

    #[test]
    fn test_foreign_keys() {
        let tables = parse_schema(SAMPLE_SQL);
        let quiz = tables.iter().find(|t| t.name == "quiz_results").unwrap();
        
        let user_id = quiz.columns.iter().find(|c| c.name == "user_id").unwrap();
        assert!(user_id.references.is_some());
        let fk = user_id.references.as_ref().unwrap();
        assert_eq!(fk.table, "profiles");
        assert_eq!(fk.column, "id");
        assert_eq!(fk.on_delete, Some("CASCADE".to_string()));
    }

    #[test]
    fn test_composite_unique() {
        let tables = parse_schema(SAMPLE_SQL);
        let usage = tables.iter().find(|t| t.name == "ai_token_usage").unwrap();
        
        assert_eq!(usage.unique_constraints.len(), 1);
        assert_eq!(usage.unique_constraints[0], vec!["user_id", "date"]);
    }

    #[test]
    fn test_check_constraints() {
        let tables = parse_schema(SAMPLE_SQL);
        let qb = tables.iter().find(|t| t.name == "question_bank").unwrap();
        
        let section = qb.columns.iter().find(|c| c.name == "section").unwrap();
        assert!(section.check_expr.is_some());
        assert!(section.check_expr.as_ref().unwrap().contains("structure"));
    }

    #[test]
    fn test_rust_type_mapping() {
        let tables = parse_schema(SAMPLE_SQL);
        let qb = tables.iter().find(|t| t.name == "question_bank").unwrap();
        
        let skill_id = qb.columns.iter().find(|c| c.name == "skill_id").unwrap();
        assert_eq!(skill_id.rust_type(), "i64");
        assert_eq!(skill_id.rust_type_full(), "i64"); // NOT NULL
        
        let difficulty = qb.columns.iter().find(|c| c.name == "difficulty_score").unwrap();
        assert_eq!(difficulty.rust_type_full(), "Option<i64>"); // nullable
        
        let passage_id = qb.columns.iter().find(|c| c.name == "passage_id").unwrap();
        assert_eq!(passage_id.rust_type_full(), "Option<String>"); // FK, nullable
        assert!(passage_id.references.is_some());
    }

    #[test]
    fn test_insertable_columns() {
        let tables = parse_schema(SAMPLE_SQL);
        let profiles = tables.iter().find(|t| t.name == "profiles").unwrap();
        
        let insertable = profiles.insertable_columns();
        // Should exclude: id (PK), created_at (auto), updated_at (auto)
        assert!(!insertable.iter().any(|c| c.name == "id"));
        assert!(!insertable.iter().any(|c| c.name == "created_at"));
        assert!(!insertable.iter().any(|c| c.name == "updated_at"));
        assert!(insertable.iter().any(|c| c.name == "username"));
        assert!(insertable.iter().any(|c| c.name == "password_hash"));
    }

    #[test]
    fn test_list_columns() {
        let tables = parse_schema(SAMPLE_SQL);
        let profiles = tables.iter().find(|t| t.name == "profiles").unwrap();
        
        let list_cols = profiles.list_columns();
        // bio could be large but "bio" doesn't match content/body/essay pattern
        // All profiles columns should be in list for this table
        assert!(list_cols.len() >= 10);
    }

    #[test]
    fn test_toefl_full_schema() {
        let sql = std::fs::read_to_string(
            "/home/abraham/Aplikasi-Ibrohim/new-toefl-quiz/src/db/migrations/001_initial_schema.sql"
        ).expect("read TOEFL schema");
        
        let tables = parse_schema(&sql);
        
        // TOEFL has ~35 tables (some may be 42 with comments, let's check)
        assert!(tables.len() >= 30, "Expected 30+ tables, got {}", tables.len());
        
        // Verify key tables exist
        let table_names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
        assert!(table_names.contains(&"profiles"), "missing profiles");
        assert!(table_names.contains(&"question_bank"), "missing question_bank");
        assert!(table_names.contains(&"quiz_results"), "missing quiz_results");
        assert!(table_names.contains(&"ai_token_usage"), "missing ai_token_usage");
        assert!(table_names.contains(&"circles"), "missing circles");
        assert!(table_names.contains(&"creators"), "missing creators");
        assert!(table_names.contains(&"blog_posts"), "missing blog_posts");
        assert!(table_names.contains(&"peer_review_submissions"), "missing peer_review_submissions");
        assert!(table_names.contains(&"app_logs"), "missing app_logs");
        assert!(table_names.contains(&"feature_flags"), "missing feature_flags");
        
        // Verify profiles has correct structure
        let profiles = tables.iter().find(|t| t.name == "profiles").unwrap();
        assert_eq!(profiles.primary_keys[0], "id");
        assert!(profiles.columns.len() >= 14);
        
        // Print summary
        println!("\n=== TOEFL Schema: {} tables ===", tables.len());
        for t in &tables {
            let pk = &t.primary_keys.join(",");
            let fks: Vec<String> = t.columns.iter()
                .filter(|c| c.references.is_some())
                .map(|c| format!("{}→{}", c.name, c.references.as_ref().unwrap().table))
                .collect();
            let uniques = t.unique_constraints.iter()
                .map(|u| format!("UNIQUE({})", u.join(",")))
                .collect::<Vec<_>>();
            println!("  {} ({} cols, pk={}{}{}) ",
                t.name, t.columns.len(), pk,
                if fks.is_empty() { String::new() } else { format!(", fk=[{}]", fks.join(",")) },
                if uniques.is_empty() { String::new() } else { format!(", {}", uniques.join(",")) },
            );
        }
    }
}
