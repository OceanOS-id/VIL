//! # VIL Migrate — SQL database migration runner
//!
//! Simple up/down migration system with tracking table.
//!
//! # Usage
//! ```ignore
//! let migrator = Migrator::new("migrations/").database_url("sqlite:data.db");
//! migrator.run().await?;    // Apply pending
//! migrator.status().await?; // Show status
//! ```

use sqlx::{Any, Pool};
type AnyPool = Pool<Any>;
use std::path::PathBuf;

/// Migration status for a single migration.
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    pub version: i64,
    pub name: String,
    pub applied: bool,
    pub applied_at: Option<String>,
}

/// Database migrator.
pub struct Migrator {
    dir: PathBuf,
    database_url: String,
}

impl Migrator {
    /// Create a new migrator pointing to a migrations directory.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            database_url: String::new(),
        }
    }

    /// Set the database URL.
    pub fn database_url(mut self, url: impl Into<String>) -> Self {
        self.database_url = url.into();
        self
    }

    /// Apply all pending migrations.
    pub async fn run(&self) -> Result<Vec<String>, MigrateError> {
        let pool = self.connect().await?;
        self.ensure_table(&pool).await?;

        let applied = self.get_applied(&pool).await?;
        let files = self.scan_up_files()?;
        let mut ran = Vec::new();

        for (version, name, path) in &files {
            if applied.contains(version) {
                continue;
            }
            let sql = std::fs::read_to_string(path)
                .map_err(|e| MigrateError::Io(format!("read {}: {e}", path.display())))?;

            // Execute each statement
            for stmt in sql.split(';') {
                let trimmed = stmt.trim();
                if trimmed.is_empty() || trimmed.starts_with("--") {
                    continue;
                }
                sqlx::query(trimmed)
                    .execute(&pool)
                    .await
                    .map_err(|e| MigrateError::Sql(format!("{name}: {e}")))?;
            }

            // Record migration
            sqlx::query(
                "INSERT INTO _vil_migrations (version, name, applied_at) VALUES (?, ?, datetime('now'))",
            )
            .bind(*version)
            .bind(name.as_str())
            .execute(&pool)
            .await
            .map_err(|e| MigrateError::Sql(e.to_string()))?;

            ran.push(name.clone());
        }

        Ok(ran)
    }

    /// Rollback the last N applied migrations.
    pub async fn rollback(&self, steps: usize) -> Result<Vec<String>, MigrateError> {
        let pool = self.connect().await?;
        self.ensure_table(&pool).await?;

        let applied = self.get_applied_ordered(&pool).await?;
        let mut rolled = Vec::new();

        for (version, name) in applied.into_iter().rev().take(steps) {
            // Find down file
            let down_path = self.dir.join(format!("{:03}_{}.down.sql", version, name));
            if down_path.exists() {
                let sql = std::fs::read_to_string(&down_path)
                    .map_err(|e| MigrateError::Io(format!("read: {e}")))?;
                for stmt in sql.split(';') {
                    let trimmed = stmt.trim();
                    if trimmed.is_empty() || trimmed.starts_with("--") { continue; }
                    sqlx::query(trimmed).execute(&pool).await
                        .map_err(|e| MigrateError::Sql(format!("{name}: {e}")))?;
                }
            }

            sqlx::query("DELETE FROM _vil_migrations WHERE version = ?")
                .bind(version)
                .execute(&pool)
                .await
                .map_err(|e| MigrateError::Sql(e.to_string()))?;

            rolled.push(name);
        }

        Ok(rolled)
    }

    /// Get migration status.
    pub async fn status(&self) -> Result<Vec<MigrationStatus>, MigrateError> {
        let pool = self.connect().await?;
        self.ensure_table(&pool).await?;

        let applied = self.get_applied(&pool).await?;
        let files = self.scan_up_files()?;
        let mut statuses = Vec::new();

        for (version, name, _) in &files {
            let is_applied = applied.contains(version);
            statuses.push(MigrationStatus {
                version: *version,
                name: name.clone(),
                applied: is_applied,
                applied_at: None, // Could query from table
            });
        }

        Ok(statuses)
    }

    async fn connect(&self) -> Result<AnyPool, MigrateError> {
        sqlx::any::install_default_drivers();
        AnyPool::connect(&self.database_url)
            .await
            .map_err(|e| MigrateError::Connection(e.to_string()))
    }

    async fn ensure_table(&self, pool: &AnyPool) -> Result<(), MigrateError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _vil_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| MigrateError::Sql(e.to_string()))?;
        Ok(())
    }

    async fn get_applied(&self, pool: &AnyPool) -> Result<Vec<i64>, MigrateError> {
        let rows = sqlx::query_as::<_, (i64,)>("SELECT version FROM _vil_migrations ORDER BY version")
            .fetch_all(pool)
            .await
            .map_err(|e| MigrateError::Sql(e.to_string()))?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    async fn get_applied_ordered(&self, pool: &AnyPool) -> Result<Vec<(i64, String)>, MigrateError> {
        let rows = sqlx::query_as::<_, (i64, String)>(
            "SELECT version, name FROM _vil_migrations ORDER BY version",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| MigrateError::Sql(e.to_string()))?;
        Ok(rows)
    }

    /// Scan migration directory for *.up.sql files.
    /// Returns sorted list of (version, name, path).
    fn scan_up_files(&self) -> Result<Vec<(i64, String, PathBuf)>, MigrateError> {
        let mut files = Vec::new();
        if !self.dir.exists() {
            return Ok(files);
        }

        for entry in std::fs::read_dir(&self.dir)
            .map_err(|e| MigrateError::Io(format!("read dir: {e}")))?
        {
            let entry = entry.map_err(|e| MigrateError::Io(e.to_string()))?;
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

            if name.ends_with(".up.sql") {
                // Parse version from filename: 001_name.up.sql
                if let Some(version_str) = name.split('_').next() {
                    if let Ok(version) = version_str.parse::<i64>() {
                        let migration_name = name
                            .trim_end_matches(".up.sql")
                            .splitn(2, '_')
                            .nth(1)
                            .unwrap_or(&name)
                            .to_string();
                        files.push((version, migration_name, path));
                    }
                }
            } else if name.ends_with(".sql") && !name.ends_with(".down.sql") {
                // Also support plain .sql files: 001_name.sql
                if let Some(version_str) = name.split('_').next() {
                    if let Ok(version) = version_str.parse::<i64>() {
                        let migration_name = name
                            .trim_end_matches(".sql")
                            .splitn(2, '_')
                            .nth(1)
                            .unwrap_or(&name)
                            .to_string();
                        files.push((version, migration_name, path));
                    }
                }
            }
        }

        files.sort_by_key(|(v, _, _)| *v);
        Ok(files)
    }
}

/// Migration error.
#[derive(Debug)]
pub enum MigrateError {
    Connection(String),
    Sql(String),
    Io(String),
}

impl std::fmt::Display for MigrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(e) => write!(f, "connection: {e}"),
            Self::Sql(e) => write!(f, "sql: {e}"),
            Self::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for MigrateError {}
