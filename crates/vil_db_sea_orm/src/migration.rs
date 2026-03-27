// =============================================================================
// VIL DB sea-orm — Migration Runner
// =============================================================================
//
// Integrates sea-orm-migration with the Admin GUI.
// Migrations can be triggered via:
//   POST /admin/plugins/vil_db_sea_orm/migrate

use serde::Serialize;

/// Migration status.
#[derive(Debug, Clone, Serialize)]
pub struct MigrationStatus {
    pub pending: Vec<String>,
    pub applied: Vec<String>,
    pub last_applied: Option<String>,
}

/// Migration runner interface.
///
/// Users implement this trait with their sea-orm-migration::MigratorTrait.
/// The Admin GUI calls run_pending() to apply migrations.
pub trait MigrationRunner: Send + Sync {
    /// List pending migrations.
    fn pending(&self) -> Vec<String>;

    /// List applied migrations.
    fn applied(&self) -> Vec<String>;

    /// Run all pending migrations.
    fn run_pending(&self) -> Result<usize, String>;

    /// Rollback last migration.
    fn rollback_last(&self) -> Result<String, String>;

    /// Get current status.
    fn status(&self) -> MigrationStatus {
        MigrationStatus {
            pending: self.pending(),
            applied: self.applied(),
            last_applied: self.applied().last().cloned(),
        }
    }
}

/// Stub migration runner (no migrations registered).
pub struct NoopMigrationRunner;

impl MigrationRunner for NoopMigrationRunner {
    fn pending(&self) -> Vec<String> { Vec::new() }
    fn applied(&self) -> Vec<String> { Vec::new() }
    fn run_pending(&self) -> Result<usize, String> { Ok(0) }
    fn rollback_last(&self) -> Result<String, String> {
        Err("No migrations to rollback".into())
    }
}
