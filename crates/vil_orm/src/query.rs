//! VilQuery — Fluent SQL builder (JOOQ-style) for process-oriented zero-copy backends.
//!
//! Design principles:
//!   - Zero allocation where possible (Cow<str>, pre-sized Vec)
//!   - Single-pass SQL generation (no intermediate AST)
//!   - Type-safe binds via VilBind trait (mixed i64/f64/String/Option)
//!   - Composable: build query in pieces, execute once
//!
//! ```ignore
//! use vil_orm::query::VilQuery;
//!
//! // SELECT with projection (no SELECT *)
//! let users = Profile::q()
//!     .select(&["id", "username", "xp"])
//!     .where_eq("subscription_tier", "premium")
//!     .order_by_desc("xp")
//!     .limit(20)
//!     .fetch_all::<ProfileSlim>(pool)
//!     .await?;
//!
//! // INSERT ON CONFLICT
//! BlogPost::q()
//!     .insert_columns(&["id", "skill_id", "title", "content"])
//!     .value("abc").value("s1").value("Hello").value("Body text")
//!     .on_conflict("skill_id")
//!     .do_update(&["title", "content"])
//!     .execute(pool)
//!     .await?;
//!
//! // UPDATE with NULL-safe optional sets
//! Profile::q()
//!     .update()
//!     .set_optional("full_name", req.full_name.as_deref())
//!     .set_optional("bio", req.bio.as_deref())
//!     .set_raw("updated_at", "datetime('now')")
//!     .where_eq("id", &claims.sub)
//!     .execute(pool)
//!     .await?;
//!
//! // JOIN
//! Profile::q()
//!     .select(&["p.id", "p.full_name", "p.avatar_url", "p.xp"])
//!     .alias("p")
//!     .join("friends f", "f.friend_id = p.id")
//!     .where_eq("f.user_id", &user_id)
//!     .fetch_all::<FriendRow>(pool)
//!     .await?;
//! ```

use crate::bind::{VilBind, VilOptF64, VilOptI64, VilOptStr};
use sqlx::any::AnyArguments;
use std::time::Instant;

/// Query operation mode.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Select,
    Insert,
    Update,
    Delete,
}

/// Fluent SQL query builder — one struct, zero-copy where possible.
/// Uses `$N` numbered placeholders for cross-database compatibility (Postgres + SQLite).
pub struct VilQuery {
    table: String,
    alias: Option<String>,
    mode: Mode,
    // SELECT
    columns: Vec<String>,
    // WHERE
    conditions: Vec<String>,
    // JOIN
    joins: Vec<String>,
    // ORDER BY
    order_clauses: Vec<String>,
    // GROUP BY
    group_cols: Vec<String>,
    // HAVING
    having: Option<String>,
    // LIMIT/OFFSET
    limit_val: Option<i64>,
    offset_val: Option<i64>,
    // INSERT
    insert_cols: Vec<String>,
    // ON CONFLICT
    conflict_col: Option<String>,
    conflict_action: ConflictAction,
    conflict_update_cols: Vec<String>,
    conflict_raw_exprs: Vec<String>,
    // UPDATE SET
    set_clauses: Vec<String>,
    // Bind values (type-erased, heap-allocated for mixed types)
    binds: Vec<Box<dyn VilBind>>,
    // Extra binds for conflict UPDATE (appended after insert binds)
    conflict_binds: Vec<Box<dyn VilBind>>,
    // Bind counter for $N numbered placeholders
    bind_counter: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConflictAction {
    None,
    DoNothing,
    DoUpdate,
}

impl VilQuery {
    /// Create a new query builder for the given table.
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            alias: None,
            mode: Mode::Select,
            columns: Vec::new(),
            conditions: Vec::new(),
            joins: Vec::new(),
            order_clauses: Vec::new(),
            group_cols: Vec::new(),
            having: None,
            limit_val: None,
            offset_val: None,
            insert_cols: Vec::new(),
            conflict_col: None,
            conflict_action: ConflictAction::None,
            conflict_update_cols: Vec::new(),
            conflict_raw_exprs: Vec::new(),
            set_clauses: Vec::new(),
            binds: Vec::new(),
            conflict_binds: Vec::new(),
            bind_counter: 0,
        }
    }

    /// Next placeholder: $1, $2, $3... (Postgres + SQLite compatible)
    fn next_placeholder(&mut self) -> String {
        self.bind_counter += 1;
        format!("${}", self.bind_counter)
    }

    // ── Mode setters ──

    /// Switch to UPDATE mode.
    pub fn update(mut self) -> Self {
        self.mode = Mode::Update;
        self
    }

    /// Switch to DELETE mode.
    pub fn delete(mut self) -> Self {
        self.mode = Mode::Delete;
        self
    }

    // ── SELECT ──

    /// Set SELECT columns. Empty = SELECT * (discouraged in production).
    pub fn select(mut self, cols: &[&str]) -> Self {
        self.mode = Mode::Select;
        self.columns = cols.iter().map(|c| c.to_string()).collect();
        self
    }

    /// Select a single scalar expression.
    /// Example: `.select_expr("COALESCE(SUM(amount), 0)")`
    pub fn select_expr(mut self, expr: &str) -> Self {
        self.mode = Mode::Select;
        self.columns = vec![expr.to_string()];
        self
    }

    /// Table alias for JOINs.
    pub fn alias(mut self, a: &str) -> Self {
        self.alias = Some(a.to_string());
        self
    }

    // ── INSERT ──

    /// Set INSERT columns.
    pub fn insert_columns(mut self, cols: &[&str]) -> Self {
        self.mode = Mode::Insert;
        self.insert_cols = cols.iter().map(|c| c.to_string()).collect();
        self
    }

    /// Bind a value for INSERT (appended in order of columns).
    pub fn value<V: VilBind + 'static>(mut self, val: V) -> Self {
        self.binds.push(Box::new(val));
        self
    }

    /// Bind an Option<String> for INSERT (NULL if None).
    pub fn value_opt_str(mut self, val: Option<String>) -> Self {
        self.binds.push(Box::new(VilOptStr(val)));
        self
    }

    /// Bind an Option<i64> for INSERT (NULL if None).
    pub fn value_opt_i64(mut self, val: Option<i64>) -> Self {
        self.binds.push(Box::new(VilOptI64(val)));
        self
    }

    /// Bind an Option<f64> for INSERT (NULL if None).
    pub fn value_opt_f64(mut self, val: Option<f64>) -> Self {
        self.binds.push(Box::new(VilOptF64(val)));
        self
    }

    /// ON CONFLICT (column) DO NOTHING.
    pub fn on_conflict_nothing(mut self, col: &str) -> Self {
        self.conflict_col = Some(col.to_string());
        self.conflict_action = ConflictAction::DoNothing;
        self
    }

    /// ON CONFLICT (column) DO UPDATE SET cols = excluded.cols.
    pub fn on_conflict(mut self, col: &str) -> Self {
        self.conflict_col = Some(col.to_string());
        self.conflict_action = ConflictAction::DoUpdate;
        self
    }

    /// Columns to update on conflict (uses excluded.col syntax).
    pub fn do_update(mut self, cols: &[&str]) -> Self {
        self.conflict_update_cols = cols.iter().map(|c| c.to_string()).collect();
        self
    }

    /// Raw expression for ON CONFLICT DO UPDATE SET. Example: `.do_update_raw("tokens_used = tokens_used + 3")`
    pub fn do_update_raw(mut self, expr: &str) -> Self {
        self.conflict_raw_exprs.push(expr.to_string());
        self
    }

    // ── UPDATE SET ──

    /// SET column = ? with a typed bind value.
    pub fn set<V: VilBind + 'static>(mut self, col: &str, val: V) -> Self {
        let ph = self.next_placeholder(); self.set_clauses.push(format!("{} = {}", col, ph));
        self.binds.push(Box::new(val));
        self
    }

    /// SET column = ? only if Some, SKIP if None. No COALESCE needed.
    pub fn set_optional(mut self, col: &str, val: Option<&str>) -> Self {
        if let Some(v) = val {
            let ph = self.next_placeholder(); self.set_clauses.push(format!("{} = {}", col, ph));
            self.binds.push(Box::new(v.to_string()));
        }
        self
    }

    /// SET column = ? with Option<i64>.
    pub fn set_optional_i64(mut self, col: &str, val: Option<i64>) -> Self {
        if let Some(v) = val {
            let ph = self.next_placeholder(); self.set_clauses.push(format!("{} = {}", col, ph));
            self.binds.push(Box::new(v));
        }
        self
    }

    /// SET column = ? with Option<f64>.
    pub fn set_optional_f64(mut self, col: &str, val: Option<f64>) -> Self {
        if let Some(v) = val {
            let ph = self.next_placeholder(); self.set_clauses.push(format!("{} = {}", col, ph));
            self.binds.push(Box::new(v));
        }
        self
    }

    /// SET with raw SQL expression (no bind). Example: `set_raw("updated_at", "datetime('now')")`
    pub fn set_raw(mut self, col: &str, expr: &str) -> Self {
        self.set_clauses.push(format!("{} = {}", col, expr));
        self
    }

    /// SET with raw expression + bind. Example: `set_expr("xp", "xp + ?", 25_i64)`
    pub fn set_expr<V: VilBind + 'static>(mut self, col: &str, expr: &str, val: V) -> Self {
        let ph = self.next_placeholder(); let fixed_expr = expr.replace("?", &ph); self.set_clauses.push(format!("{} = {}", col, fixed_expr));
        self.binds.push(Box::new(val));
        self
    }

    // ── WHERE ──

    /// WHERE column = ? (string bind).
    pub fn where_eq(mut self, col: &str, val: &str) -> Self {
        let ph = self.next_placeholder(); self.conditions.push(format!("{} = {}", col, ph));
        self.binds.push(Box::new(val.to_string()));
        self
    }

    /// WHERE column = ? (typed bind).
    pub fn where_eq_val<V: VilBind + 'static>(mut self, col: &str, val: V) -> Self {
        let ph = self.next_placeholder(); self.conditions.push(format!("{} = {}", col, ph));
        self.binds.push(Box::new(val));
        self
    }

    /// WHERE column != ?
    pub fn where_ne(mut self, col: &str, val: &str) -> Self {
        let ph = self.next_placeholder(); self.conditions.push(format!("{} != {}", col, ph));
        self.binds.push(Box::new(val.to_string()));
        self
    }

    /// WHERE column > ? (typed).
    pub fn where_gt<V: VilBind + 'static>(mut self, col: &str, val: V) -> Self {
        let ph = self.next_placeholder(); self.conditions.push(format!("{} > {}", col, ph));
        self.binds.push(Box::new(val));
        self
    }

    /// WHERE column < ? (typed).
    pub fn where_lt<V: VilBind + 'static>(mut self, col: &str, val: V) -> Self {
        let ph = self.next_placeholder(); self.conditions.push(format!("{} < {}", col, ph));
        self.binds.push(Box::new(val));
        self
    }

    /// WHERE column >= ? (typed).
    pub fn where_gte<V: VilBind + 'static>(mut self, col: &str, val: V) -> Self {
        let ph = self.next_placeholder(); self.conditions.push(format!("{} >= {}", col, ph));
        self.binds.push(Box::new(val));
        self
    }

    /// WHERE column IS NULL.
    pub fn where_null(mut self, col: &str) -> Self {
        self.conditions.push(format!("{} IS NULL", col));
        self
    }

    /// WHERE column IS NOT NULL.
    pub fn where_not_null(mut self, col: &str) -> Self {
        self.conditions.push(format!("{} IS NOT NULL", col));
        self
    }

    /// WHERE raw SQL fragment with binds.
    /// Example: `.where_raw("created_at > datetime('now', '-1 hour')")`
    pub fn where_raw(mut self, sql: &str) -> Self {
        self.conditions.push(sql.to_string());
        self
    }

    /// WHERE raw SQL with a typed bind.
    pub fn where_raw_bind<V: VilBind + 'static>(mut self, sql: &str, val: V) -> Self {
        let ph = self.next_placeholder(); self.conditions.push(sql.replace("?", &ph));
        self.binds.push(Box::new(val));
        self
    }

    /// AND column = ? (alias for where_eq, for readability in chains).
    pub fn and_eq(self, col: &str, val: &str) -> Self {
        self.where_eq(col, val)
    }

    // ── JOIN ──

    /// INNER JOIN table ON condition.
    pub fn join(mut self, table_alias: &str, on: &str) -> Self {
        self.joins.push(format!("JOIN {} ON {}", table_alias, on));
        self
    }

    /// LEFT JOIN table ON condition.
    pub fn left_join(mut self, table_alias: &str, on: &str) -> Self {
        self.joins.push(format!("LEFT JOIN {} ON {}", table_alias, on));
        self
    }

    // ── ORDER BY / GROUP BY / LIMIT ──

    pub fn order_by_asc(mut self, col: &str) -> Self {
        self.order_clauses.push(format!("{} ASC", col));
        self
    }

    pub fn order_by_desc(mut self, col: &str) -> Self {
        self.order_clauses.push(format!("{} DESC", col));
        self
    }

    /// ORDER BY raw expression. Example: `.order_by_raw("RANDOM()")`
    pub fn order_by_raw(mut self, expr: &str) -> Self {
        self.order_clauses.push(expr.to_string());
        self
    }

    pub fn group_by(mut self, col: &str) -> Self {
        self.group_cols.push(col.to_string());
        self
    }

    pub fn having(mut self, expr: &str) -> Self {
        self.having = Some(expr.to_string());
        self
    }

    pub fn limit(mut self, n: i64) -> Self {
        self.limit_val = Some(n);
        self
    }

    pub fn offset(mut self, n: i64) -> Self {
        self.offset_val = Some(n);
        self
    }

    // ── SQL generation (single-pass, zero intermediate AST) ──

    /// Build the final SQL string. Call this to inspect generated SQL.
    pub fn to_sql(&self) -> String {
        match self.mode {
            Mode::Select => self.build_select(),
            Mode::Insert => self.build_insert(),
            Mode::Update => self.build_update(),
            Mode::Delete => self.build_delete(),
        }
    }

    fn table_ref(&self) -> String {
        match &self.alias {
            Some(a) => format!("{} {}", self.table, a),
            None => self.table.clone(),
        }
    }

    fn build_select(&self) -> String {
        let cols = if self.columns.is_empty() {
            "*".to_string()
        } else {
            self.columns.join(", ")
        };

        let mut sql = format!("SELECT {} FROM {}", cols, self.table_ref());

        for j in &self.joins {
            sql.push(' ');
            sql.push_str(j);
        }

        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.conditions.join(" AND "));
        }

        if !self.group_cols.is_empty() {
            sql.push_str(" GROUP BY ");
            sql.push_str(&self.group_cols.join(", "));
        }

        if let Some(ref h) = self.having {
            sql.push_str(" HAVING ");
            sql.push_str(h);
        }

        if !self.order_clauses.is_empty() {
            sql.push_str(" ORDER BY ");
            sql.push_str(&self.order_clauses.join(", "));
        }

        if let Some(l) = self.limit_val {
            sql.push_str(&format!(" LIMIT {}", l));
        }
        if let Some(o) = self.offset_val {
            sql.push_str(&format!(" OFFSET {}", o));
        }

        sql
    }

    fn build_insert(&self) -> String {
        let placeholders: Vec<String> = (1..=self.insert_cols.len())
            .map(|i| format!("${}", i))
            .collect();
        let mut sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.table,
            self.insert_cols.join(", "),
            placeholders.join(", ")
        );

        if let Some(ref col) = self.conflict_col {
            match self.conflict_action {
                ConflictAction::DoNothing => {
                    sql.push_str(&format!(" ON CONFLICT({}) DO NOTHING", col));
                }
                ConflictAction::DoUpdate => {
                    let mut updates: Vec<String> = self
                        .conflict_update_cols
                        .iter()
                        .map(|c| format!("{} = excluded.{}", c, c))
                        .collect();
                    updates.extend(self.conflict_raw_exprs.iter().cloned());
                    sql.push_str(&format!(
                        " ON CONFLICT({}) DO UPDATE SET {}",
                        col,
                        updates.join(", ")
                    ));
                }
                ConflictAction::None => {}
            }
        }

        sql
    }

    fn build_update(&self) -> String {
        let mut sql = format!("UPDATE {} SET {}", self.table, self.set_clauses.join(", "));

        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.conditions.join(" AND "));
        }

        sql
    }

    fn build_delete(&self) -> String {
        let mut sql = format!("DELETE FROM {}", self.table);

        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.conditions.join(" AND "));
        }

        sql
    }

    /// Build AnyArguments from accumulated binds.
    fn build_args(&self) -> AnyArguments<'_> {
        let mut args = AnyArguments::default();

        // For UPDATE mode: SET binds come first (in self.binds),
        // then WHERE binds. But we accumulate them in order already
        // since set() pushes to binds, then where_eq() pushes to binds.
        //
        // For INSERT: binds are values, then conflict_binds (if any).
        for b in &self.binds {
            b.bind_to(&mut args);
        }
        for b in &self.conflict_binds {
            b.bind_to(&mut args);
        }

        args
    }

    // ── db_log helper ──

    fn op_type_code(&self) -> u8 {
        match self.mode {
            Mode::Select => 0,
            Mode::Insert => 1,
            Mode::Update => 2,
            Mode::Delete => 3,
        }
    }

    fn emit_db_log(&self, sql: &str, duration_ns: u64, rows: u32, error_code: u8) {
        let table_hash = vil_log::dict::register_str(&self.table);
        let query_hash = vil_log::dict::register_str(sql);
        vil_log::db_log!(Info, vil_log::DbPayload {
            db_hash: 0,
            table_hash,
            query_hash,
            rows_affected: rows,
            duration_ns,
            op_type: self.op_type_code(),
            prepared: 1,
            tx_state: 0,
            error_code,
            pool_id: 0,
            shard_id: 0,
            meta_bytes: [0; 160],
        });
    }

    // ── Terminal operations (execute against pool) ──

    /// Fetch all rows as Vec<T>.
    pub async fn fetch_all<T: for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> + Send + Unpin>(
        self,
        pool: &sqlx::Pool<sqlx::Any>,
    ) -> Result<Vec<T>, sqlx::Error> {
        let sql = self.to_sql();
        let args = self.build_args();
        let start = Instant::now();
        let result = sqlx::query_as_with::<_, T, _>(&sql, args)
            .fetch_all(pool)
            .await;
        let dur = start.elapsed().as_nanos() as u64;
        match &result {
            Ok(rows) => self.emit_db_log(&sql, dur, rows.len() as u32, 0),
            Err(_) => self.emit_db_log(&sql, dur, 0, 1),
        }
        result
    }

    /// Fetch one row (error if not found).
    pub async fn fetch_one<T: for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> + Send + Unpin>(
        self,
        pool: &sqlx::Pool<sqlx::Any>,
    ) -> Result<T, sqlx::Error> {
        let sql = self.to_sql();
        let args = self.build_args();
        let start = Instant::now();
        let result = sqlx::query_as_with::<_, T, _>(&sql, args)
            .fetch_one(pool)
            .await;
        let dur = start.elapsed().as_nanos() as u64;
        self.emit_db_log(&sql, dur, if result.is_ok() { 1 } else { 0 }, result.is_err() as u8);
        result
    }

    /// Fetch optional row.
    pub async fn fetch_optional<T: for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> + Send + Unpin>(
        self,
        pool: &sqlx::Pool<sqlx::Any>,
    ) -> Result<Option<T>, sqlx::Error> {
        let sql = self.to_sql();
        let args = self.build_args();
        let start = Instant::now();
        let result = sqlx::query_as_with::<_, T, _>(&sql, args)
            .fetch_optional(pool)
            .await;
        let dur = start.elapsed().as_nanos() as u64;
        match &result {
            Ok(Some(_)) => self.emit_db_log(&sql, dur, 1, 0),
            Ok(None) => self.emit_db_log(&sql, dur, 0, 0),
            Err(_) => self.emit_db_log(&sql, dur, 0, 1),
        }
        result
    }

    /// Fetch a single scalar value.
    pub async fn scalar<T: sqlx::Type<sqlx::Any> + for<'r> sqlx::Decode<'r, sqlx::Any> + Send + Unpin>(
        self,
        pool: &sqlx::Pool<sqlx::Any>,
    ) -> Result<T, sqlx::Error> {
        let sql = self.to_sql();
        let args = self.build_args();
        let start = Instant::now();
        let result = sqlx::query_scalar_with::<_, T, _>(&sql, args)
            .fetch_one(pool)
            .await;
        let dur = start.elapsed().as_nanos() as u64;
        self.emit_db_log(&sql, dur, 1, result.is_err() as u8);
        result
    }

    /// Fetch an optional scalar value.
    pub async fn scalar_optional<T: sqlx::Type<sqlx::Any> + for<'r> sqlx::Decode<'r, sqlx::Any> + Send + Unpin>(
        self,
        pool: &sqlx::Pool<sqlx::Any>,
    ) -> Result<Option<T>, sqlx::Error> {
        let sql = self.to_sql();
        let args = self.build_args();
        let start = Instant::now();
        let result = sqlx::query_scalar_with::<_, T, _>(&sql, args)
            .fetch_optional(pool)
            .await;
        let dur = start.elapsed().as_nanos() as u64;
        match &result {
            Ok(Some(_)) => self.emit_db_log(&sql, dur, 1, 0),
            Ok(None) => self.emit_db_log(&sql, dur, 0, 0),
            Err(_) => self.emit_db_log(&sql, dur, 0, 1),
        }
        result
    }

    /// Execute (INSERT/UPDATE/DELETE). Returns rows affected.
    /// UPDATE with no SET clauses returns 0 (no-op) instead of invalid SQL.
    pub async fn execute(
        self,
        pool: &sqlx::Pool<sqlx::Any>,
    ) -> Result<u64, sqlx::Error> {
        // Guard: UPDATE with empty SET → no-op
        if self.mode == Mode::Update && self.set_clauses.is_empty() {
            return Ok(0);
        }
        let sql = self.to_sql();
        let args = self.build_args();
        let start = Instant::now();
        let result = sqlx::query_with(&sql, args)
            .execute(pool)
            .await;
        let dur = start.elapsed().as_nanos() as u64;
        match &result {
            Ok(r) => self.emit_db_log(&sql, dur, r.rows_affected() as u32, 0),
            Err(_) => self.emit_db_log(&sql, dur, 0, 1),
        }
        result.map(|r| r.rows_affected())
    }
}
